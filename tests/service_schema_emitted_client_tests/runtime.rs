//! What the runtime groups share: a workspace per run, the run itself, and the stand-down a
//! machine with no such runtime takes.

use core::sync::atomic::{AtomicU32, Ordering};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::io::Write as _;
use std::io::stderr;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, id};
use std::sync::Mutex;

/// The process id alone does not separate two tests running beside each other.
static RUNS: AtomicU32 = AtomicU32::new(0);

/// The keys already reported absent: once per key, so two silent groups are not one.
static STOOD_DOWN: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// A directory whose `node_modules` holds the packages a run imports; symlinked into the
/// workspace by [`ran_with_modules`], since Node's ESM resolver does not consult `NODE_PATH`.
pub const NODE_MODULES_VAR: &str = "TIXSCHEMA_NODE_MODULES";

/// Names the compiler, the runtime, and the directory holding the serialization compiler plugin
/// jar and the `kotlinx-serialization-json`, `kotlinx-serialization-core` and
/// `kotlinx-coroutines-core` jars a Kotlin toolchain needs.
#[cfg(feature = "kotlin")]
pub const KOTLINC_VAR: &str = "TIXSCHEMA_KOTLINC";
#[cfg(feature = "kotlin")]
pub const JAVA_VAR: &str = "TIXSCHEMA_JAVA";
#[cfg(feature = "kotlin")]
pub const KOTLIN_LIBS_VAR: &str = "TIXSCHEMA_KOTLIN_LIBS";

/// What [`kotlin_toolchain`] found: the compiler and runtime [`ran_kotlin`] invokes, the plugin
/// jar, and the classpath the three library jars join into.
#[cfg(feature = "kotlin")]
struct KotlinToolchain {
    classpath: String,
    java: String,
    kotlinc: String,
    plugin: PathBuf,
}

/// `None` unless every package in `required` has a directory under `TIXSCHEMA_NODE_MODULES`'s own
/// `node_modules`.
pub fn node_modules(required: &[&str]) -> Option<PathBuf> {
    let at = PathBuf::from(env::var(NODE_MODULES_VAR).ok()?).join("node_modules");
    required
        .iter()
        .all(|package| at.join(package).is_dir())
        .then_some(at)
}

/// A directory of its own per run.
fn workspace(named: &str) -> PathBuf {
    let nth = RUNS.fetch_add(1, Ordering::Relaxed);
    let at = temp_dir().join(format!("tixschema-emitted-{named}-{run}-{nth}", run = id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(&at).unwrap();
    at
}

/// Writes `entry` into `at`, runs it under the named runtime, and answers what the process wrote
/// to stdout.
///
/// `None` says no runtime was reachable and nothing ran — never that a run passed. A runtime named
/// explicitly in `var` that cannot be started is a failure instead.
fn run_in(
    var: &str,
    fallback: &'static str,
    entry: &str,
    source: &str,
    at: &Path,
) -> Option<String> {
    let chosen = env::var(var).ok();
    let runtime = chosen.clone().unwrap_or_else(|| fallback.to_owned());
    fs::write(at.join(entry), source).unwrap();
    let run = Command::new(&runtime).arg(entry).current_dir(at).output();
    let Ok(reported) = run else {
        assert!(
            chosen.is_none(),
            "{var} names `{runtime}`, and no runtime could be started there: {}",
            run.unwrap_err()
        );
        stand_down(var, fallback);
        fs::remove_dir_all(at).unwrap();
        return None;
    };
    fs::remove_dir_all(at).unwrap();
    assert!(
        reported.status.success(),
        "`{runtime} {entry}` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        String::from_utf8_lossy(&reported.stdout),
        String::from_utf8_lossy(&reported.stderr)
    );
    Some(String::from_utf8_lossy(&reported.stdout).into_owned())
}

/// Writes `entry` into a workspace of its own and runs it under the named runtime. See [`run_in`].
pub fn ran(
    named: &str,
    var: &str,
    fallback: &'static str,
    entry: &str,
    source: &str,
) -> Option<String> {
    let at = workspace(named);
    run_in(var, fallback, entry, source, &at)
}

/// Like [`ran`], but first symlinks `<workspace>/node_modules` to `modules` (as returned by
/// [`node_modules`]) so the entry's ESM imports resolve — Node's ESM resolver ignores `NODE_PATH`.
pub fn ran_with_modules(
    named: &str,
    var: &str,
    fallback: &'static str,
    entry: &str,
    source: &str,
    modules: &Path,
) -> Option<String> {
    let at = workspace(named);
    symlink(modules, at.join("node_modules")).unwrap();
    run_in(var, fallback, entry, source, &at)
}

/// Says `notice` on the process's own stderr, which `cargo test` does not capture — but only the
/// first time for a given `key`, so two silent groups sharing one reason are not reported twice.
fn once(key: &'static str, notice: &str) {
    let already = {
        let mut said = STOOD_DOWN.lock().unwrap();
        let seen = said.contains(&key);
        if !seen {
            said.push(key);
        }
        seen
    };
    if !already {
        drop(stderr().write_all(notice.as_bytes()));
    }
}

/// Said when no runtime named by `var`, nor `fallback` on `PATH`, could be started.
fn stand_down(var: &str, fallback: &'static str) {
    once(
        fallback,
        &format!(
            "\ntixschema: no `{fallback}` is reachable, so the emitted client was NOT run.\n  \
             That group stood down. Put `{fallback}` on PATH, or name one in {var}, and run \
             `just test-emitted`, which refuses to stand down.\n\n"
        ),
    );
}

/// Said when [`node_modules`] found no directory holding every package in `required`, naming the
/// `surface` that stood down (e.g. "the emitted WebSocket server").
pub fn stand_down_modules(required: &[&str], surface: &str) {
    let packages = required.join(", ");
    once(
        NODE_MODULES_VAR,
        &format!(
            "\ntixschema: no `{packages}` package is reachable through {NODE_MODULES_VAR}, so \
             {surface} was NOT run.\n  That group stood down. Set {NODE_MODULES_VAR} to a \
             directory whose `node_modules` holds {packages}, and run `just test-emitted`, which \
             refuses to stand down.\n\n"
        ),
    );
}

// Kotlin: a two-step compile-then-run, and a classpath assembled from a directory rather than
// a single named binary, since `kotlinx.serialization` needs its own compiler plugin.

/// Says a Kotlin-toolchain notice on stderr, once per run — reuses [`once`]'s own key ("kotlinc")
/// for every reason a Kotlin group stands down, so the environment is told once rather than once
/// per missing piece per test.
#[cfg(feature = "kotlin")]
fn kotlin_stand_down(notice: &str) {
    once("kotlinc", notice);
}

/// The jar under `libs` whose file name starts with `stem`; `None`, with a notice naming `stem`,
/// if `libs` is not a directory or holds none.
#[cfg(feature = "kotlin")]
fn kotlin_jar(libs: &Path, stem: &str) -> Option<PathBuf> {
    let found = fs::read_dir(libs)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(stem))
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        });
    if found.is_none() {
        kotlin_stand_down(&format!(
            "\ntixschema: no `{stem}*.jar` under {KOTLIN_LIBS_VAR}, so the emitted Kotlin was NOT \
             run.\n  Run `just kotlin-libs` to install it, then `just test-emitted`, which refuses \
             to stand down.\n\n"
        ));
    }
    found
}

/// `None` unless `TIXSCHEMA_KOTLIN_LIBS` names a directory holding the serialization compiler
/// plugin jar and the three library jars the emitted Kotlin imports.
#[cfg(feature = "kotlin")]
fn kotlin_toolchain() -> Option<KotlinToolchain> {
    let Ok(named_libs) = env::var(KOTLIN_LIBS_VAR) else {
        kotlin_stand_down(&format!(
            "\ntixschema: {KOTLIN_LIBS_VAR} is not set, so the emitted Kotlin was NOT run.\n  Run \
             `just kotlin-libs`, which installs the jars into ~/.local/share/tixschema/kotlin-libs, \
             then `just test-emitted`, which reads that directory and refuses to stand down.\n\n"
        ));
        return None;
    };
    let libs = PathBuf::from(named_libs);
    let plugin = kotlin_jar(&libs, "kotlinx-serialization-compiler-plugin")?;
    let mut jars = Vec::new();
    for stem in [
        "kotlinx-serialization-json",
        "kotlinx-serialization-core",
        "kotlinx-coroutines-core",
    ] {
        jars.push(kotlin_jar(&libs, stem)?);
    }
    let classpath = jars
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    Some(KotlinToolchain {
        classpath,
        java: env::var(JAVA_VAR).unwrap_or_else(|_| "java".to_owned()),
        kotlinc: env::var(KOTLINC_VAR).unwrap_or_else(|_| "kotlinc".to_owned()),
        plugin,
    })
}

/// Compiles `source` as `main.kt` against the three library jars and runs the resulting jar
/// under `java`. `None` says no toolchain piece was reachable; a piece named explicitly that
/// still fails to start is a failure instead, matching [`run_in`]'s rule.
#[cfg(feature = "kotlin")]
pub fn ran_kotlin(source: &str) -> Option<String> {
    let toolchain = kotlin_toolchain()?;
    let at = workspace("kotlin");
    fs::write(at.join("main.kt"), source).unwrap();

    let named_kotlinc = env::var(KOTLINC_VAR).ok();
    let compile = Command::new(&toolchain.kotlinc)
        .arg(format!("-Xplugin={}", toolchain.plugin.display()))
        .args([
            "-cp",
            &toolchain.classpath,
            "main.kt",
            "-include-runtime",
            "-d",
            "main.jar",
        ])
        .current_dir(&at)
        .output();
    let Ok(compiled) = compile else {
        let error = compile.unwrap_err();
        assert!(
            named_kotlinc.is_none(),
            "{KOTLINC_VAR} names `{}`, and no compiler could be started there: {error}",
            toolchain.kotlinc
        );
        kotlin_stand_down(&format!(
            "\ntixschema: no `kotlinc` is reachable, so the emitted Kotlin was NOT run.\n  Put \
             `kotlinc` on PATH, or name one in {KOTLINC_VAR}, and run `just test-emitted`, which \
             refuses to stand down.\n\n"
        ));
        fs::remove_dir_all(&at).unwrap();
        return None;
    };
    assert!(
        compiled.status.success(),
        "`{} main.kt` failed to compile.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        toolchain.kotlinc,
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );

    let named_java = env::var(JAVA_VAR).ok();
    let run_classpath = format!("main.jar:{}", toolchain.classpath);
    let run_attempt = Command::new(&toolchain.java)
        .args(["-cp", &run_classpath, "MainKt"])
        .current_dir(&at)
        .output();
    let Ok(run) = run_attempt else {
        let error = run_attempt.unwrap_err();
        assert!(
            named_java.is_none(),
            "{JAVA_VAR} names `{}`, and no runtime could be started there: {error}",
            toolchain.java
        );
        kotlin_stand_down(&format!(
            "\ntixschema: no `java` is reachable, so the emitted Kotlin was NOT run.\n  Put \
             `java` on PATH, or name one in {JAVA_VAR}, and run `just test-emitted`, which \
             refuses to stand down.\n\n"
        ));
        fs::remove_dir_all(&at).unwrap();
        return None;
    };
    fs::remove_dir_all(&at).unwrap();
    assert!(
        run.status.success(),
        "`{} -cp main.jar MainKt` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        toolchain.java,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    Some(String::from_utf8_lossy(&run.stdout).into_owned())
}
