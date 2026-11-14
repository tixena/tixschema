//! The constants a valid type holds for its unit variants and its unit struct, and the infallible
//! `From` of a draft that declares no rule.

#![cfg(feature = "serde")]

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Counted {
        Closed,
        Empty,
        Named { label: String },
        Seen(u32),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Format {
        A,
        B,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SessionRole {
        Auditor,
        DntiTechnician,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct RuleResult {
        pub label: String,
        pub passed: bool,
        pub role: SessionRole,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SealAck;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Tally<T> {
        pub held: T,
    }

    impl SessionRole {
        const fn can_sign(self) -> bool {
            matches!(self, Self::DntiTechnician)
        }
    }

    const fn default_role() -> SessionRole {
        SessionRole::DntiTechnician
    }

    const fn code(role: SessionRole) -> u8 {
        match role {
            SessionRole::DntiTechnician => 1,
            SessionRole::Auditor => 2,
        }
    }

    fn answer(refuse: bool) -> Result<SealAck, String> {
        if refuse {
            Err("refused".to_owned())
        } else {
            Ok(SealAck)
        }
    }

    #[test]
    fn a_unit_variant_is_a_constant_of_the_valid_type() {
        let role: SessionRole = SessionRole::DntiTechnician;
        assert_eq!(role, default_role());
        assert!(role.can_sign());
        assert!(!SessionRole::Auditor.can_sign());
        assert_eq!(
            SessionRole::DntiTechnician,
            SessionRoleDraft::DntiTechnician.validate().unwrap()
        );
    }

    #[test]
    fn a_match_on_every_constant_needs_no_wildcard_arm() {
        assert_eq!(code(SessionRole::DntiTechnician), 1);
        assert_eq!(code(SessionRole::Auditor), 2);
    }

    #[test]
    fn a_constant_writes_the_payload_of_its_variant() {
        assert_eq!(
            serde_json::to_string(&SessionRole::Auditor).unwrap(),
            "\"Auditor\""
        );
    }

    #[test]
    fn only_the_unit_variants_of_a_mixed_enum_are_constants() {
        assert_eq!(serde_json::to_string(&Counted::Empty).unwrap(), "\"Empty\"");
        assert_eq!(
            serde_json::to_string(&Counted::Closed).unwrap(),
            "\"Closed\""
        );
        let seen: Counted = CountedDraft::Seen(3).into();
        assert_eq!(serde_json::to_string(&seen).unwrap(), "{\"Seen\":3}");
    }

    #[test]
    fn a_unit_struct_is_a_constant_of_the_valid_type() {
        assert_eq!(answer(false), Ok(SealAck));
        assert_eq!(answer(true), Err("refused".to_owned()));
        assert_eq!(serde_json::to_string(&SealAck).unwrap(), "{}");
    }

    #[test]
    fn a_draft_with_no_rule_converts_with_from() {
        let rule: RuleResult = RuleResultDraft {
            label: "sealed".to_owned(),
            passed: true,
            role: SessionRole::DntiTechnician,
        }
        .into();
        assert!(rule.passed);
        assert_eq!(rule.role, SessionRole::DntiTechnician);
        assert_eq!(rule.label, "sealed");
        let counted: Counted = CountedDraft::Seen(1).into();
        assert_eq!(serde_json::to_string(&counted).unwrap(), "{\"Seen\":1}");
        let tally: Tally<u8> = TallyDraft { held: 4 }.into();
        assert_eq!(tally.held, 4);
    }

    #[test]
    fn an_enum_of_upper_case_variants_has_constants_too() {
        assert_eq!(serde_json::to_string(&Format::B).unwrap(), "\"B\"");
        assert_ne!(Format::A, Format::B);
    }
}
