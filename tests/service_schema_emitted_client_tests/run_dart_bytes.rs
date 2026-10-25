//! A `Vec<u8>` read and written by the emitted Dart types under the Dart SDK: the JSON array serde
//! writes is read into a `Uint8List`, an item outside a byte is refused in the words of today, and
//! what the types write is the JSON serde writes. The analyzer passes the emitted types.

#![cfg(all(feature = "dart", feature = "serde"))]

use std::collections::HashMap;

use super::bytes_models::{
    BytesCounted, BytesEither, BytesExample, BytesHolder, BytesImage, BytesLookalikes,
    BytesOptional, bytes_box_dart, bytes_counted_dart, bytes_either_dart, bytes_holder_dart,
    bytes_image_dart, bytes_lookalikes_dart, bytes_optional_dart, bytes_payload_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a read: the message of its refusal, or the JSON it writes back.
const ROW: &str = "
void row(String name, Object? Function() read) {
  try {
    print('$name: ${jsonEncode(read())}');
  } on FormatException catch (refused) {
    print('$name: refused ${refused.message}');
  }
}
";

/// The emitted types of the models, beside the import the generated Dart file needs for bytes.
fn definitions() -> Vec<String> {
    vec![
        bytes_payload_dart::dart_definition(),
        bytes_box_dart::dart_definition(),
        bytes_image_dart::dart_definition(),
        bytes_holder_dart::dart_definition(),
        bytes_counted_dart::dart_definition(),
        bytes_either_dart::dart_definition(),
        bytes_optional_dart::dart_definition(),
        bytes_lookalikes_dart::dart_definition(),
    ]
}

/// The holder serde writes the JSON of.
fn holder() -> BytesHolder {
    BytesHolder {
        aliased: vec![9],
        data: vec![137, 80, 78, 71],
        filled: super::bytes_models::BytesBox { held: vec![0, 255] },
        keyed: HashMap::from([("k".to_owned(), vec![1, 2])]),
        outcome: Ok(vec![4, 5]),
        paired: (vec![6], "x".to_owned()),
        rows: vec![vec![7], vec![]],
    }
}

/// The Dart `main`: each payload is read with the `fromJson` of its type.
fn main_source(holder_json: &str) -> String {
    format!(
        "void main() {{
  final image = BytesImage.fromJson(jsonDecode('{{\"contentType\":\"image/png\",\"data\":[137,80,78,71]}}'));
  print('uint8list: ${{image.data is Uint8List}} ${{image.data}}');
  row('image', () => image);
  row('empty', () => BytesImage.fromJson(jsonDecode('{{\"contentType\":\"a\",\"data\":[]}}')));
  row('too large', () => BytesImage.fromJson(jsonDecode('{{\"contentType\":\"a\",\"data\":[256]}}')));
  row('negative', () => BytesImage.fromJson(jsonDecode('{{\"contentType\":\"a\",\"data\":[-1]}}')));
  row('holder', () => BytesHolder.fromJson(jsonDecode(r'{holder_json}')));
  row('counted', () => BytesCounted.fromJson(jsonDecode('{{\"data\":[1,2,3]}}')));
  row('too short', () => BytesCounted.fromJson(jsonDecode('{{\"data\":[1]}}')));
  row('too long', () => BytesCounted.fromJson(jsonDecode('{{\"data\":[1,2,3,4,5]}}')));
  row('absent', () => BytesOptional.fromJson(jsonDecode('{{}}')));
  row('present', () => BytesOptional.fromJson(jsonDecode('{{\"maybe\":[3]}}')));
  row('null', () => BytesOptional.fromJson(jsonDecode('{{\"maybe\":null}}')));
  row('either bytes', () => BytesEither.fromJson(jsonDecode('[1,2]')));
  row('either text', () => BytesEither.fromJson(jsonDecode('\"x\"')));
  final built = BytesImage(content_type: 'a', data: Uint8List.fromList([1, 2]));
  row('built', () => built);
  final looked = BytesLookalikes.fromJson(jsonDecode('{{\"boxed\":[1],\"digest\":[1,2,3,4],\"distinct\":[1],\"gaps\":[null,2],\"queued\":[1],\"sevens\":[7],\"signed\":[-1],\"wide\":[300]}}'));
  print('lookalikes: ${{looked.boxed is Uint8List}} ${{looked.sevens is Uint8List}} ${{looked.wide}}');
}}
"
    )
}

/// Each payload is read into the bytes it holds and written back as the JSON serde writes, and an
/// item outside a byte is refused with the sentence a `List<int>` was refused with.
#[test]
fn a_vec_of_u8_is_read_into_a_uint8list_and_written_as_the_json_array() {
    let written = serde_json::to_string(&holder()).unwrap();
    let source = module(&definitions(), &[ROW], &main_source(&written));
    let Some((ran, printed)) = dart("bytes", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        [
            "uint8list: true [137, 80, 78, 71]".to_owned(),
            r#"image: {"contentType":"image/png","data":[137,80,78,71]}"#.to_owned(),
            r#"empty: {"contentType":"a","data":[]}"#.to_owned(),
            "too large: refused expected an integer from 0 to 255, found 256".to_owned(),
            "negative: refused expected an integer from 0 to 255, found -1".to_owned(),
            format!("holder: {written}"),
            r#"counted: {"data":[1,2,3]}"#.to_owned(),
            "too short: refused 'data': too short: minimum length is 2, got 1".to_owned(),
            "too long: refused 'data': too long: maximum length is 4, got 5".to_owned(),
            "absent: {}".to_owned(),
            r#"present: {"maybe":[3]}"#.to_owned(),
            "null: {}".to_owned(),
            "either bytes: [1,2]".to_owned(),
            r#"either text: "x""#.to_owned(),
            r#"built: {"contentType":"a","data":[1,2]}"#.to_owned(),
            "lookalikes: false false [300]".to_owned(),
            String::new(),
        ]
        .join("\n")
    );
    assert_eq!(
        serde_json::from_str::<BytesImage>(r#"{"contentType":"a","data":[1,2]}"#)
            .unwrap()
            .data,
        vec![1, 2]
    );
    assert_eq!(
        BytesCounted {
            data: vec![1, 2, 3]
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        serde_json::from_str::<BytesOptional>(r#"{"maybe":null}"#).unwrap(),
        BytesOptional { maybe: None }
    );
    assert_eq!(
        serde_json::from_str::<BytesEither>("[1,2]").unwrap(),
        BytesEither::Bytes(vec![1, 2])
    );
    assert_eq!(
        serde_json::to_string(&BytesExample { data: vec![1] }).unwrap(),
        r#"{"data":[1]}"#
    );
    let looked = r#"{"boxed":[1],"digest":[1,2,3,4],"distinct":[1],"gaps":[null,2],"queued":[1],"sevens":[7],"signed":[-1],"wide":[300]}"#;
    assert_eq!(
        serde_json::to_string(&serde_json::from_str::<BytesLookalikes>(looked).unwrap()).unwrap(),
        looked
    );
}

/// The analyzer passes the emitted types.
#[test]
fn the_emitted_bytes_have_no_issue() {
    let analyzed = module(
        &definitions(),
        &[ROW],
        "void main() => row('a list', () => jsonDecode('[]'));\n",
    );
    let Some((clean, said)) = dart("bytes-analyze", &["analyze"], &analyzed) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
