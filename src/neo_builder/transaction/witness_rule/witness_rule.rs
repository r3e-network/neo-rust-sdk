use serde::{Deserialize, Serialize};

use crate::{
	builder::{TransactionError, WitnessAction, WitnessCondition},
	codec::{Decoder, Encoder, NeoSerializable},
};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Clone)]
pub struct WitnessRule {
	#[serde(rename = "action")]
	pub action: WitnessAction,
	#[serde(rename = "condition")]
	pub condition: WitnessCondition,
}

impl WitnessRule {
	pub fn new(action: WitnessAction, condition: WitnessCondition) -> Self {
		Self { action, condition }
	}

	pub fn try_encode(&self, writer: &mut Encoder) -> Result<(), TransactionError> {
		writer.write_u8(self.action as u8);
		self.condition.try_encode(writer)?;
		Ok(())
	}

	pub fn try_to_array(&self) -> Result<Vec<u8>, TransactionError> {
		let mut writer = Encoder::new();
		self.try_encode(&mut writer)?;
		Ok(writer.to_bytes())
	}
}

impl NeoSerializable for WitnessRule {
	type Error = TransactionError;

	fn size(&self) -> usize {
		1 + self.condition.size()
	}

	fn encode(&self, writer: &mut Encoder) {
		if let Err(err) = self.try_encode(writer) {
			tracing::warn!(
				error = ?err,
				"Failed to serialize witness rule via safe path; falling back to legacy encoder"
			);
			writer.write_u8(self.action as u8);
			writer.write_serializable_fixed(&self.condition);
		}
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error> {
		let action = reader.read_u8_safe()?;
		let action =
			WitnessAction::try_from(action).map_err(|_| TransactionError::InvalidTransaction)?;
		let condition = WitnessCondition::decode(reader)?;
		Ok(Self { action, condition })
	}
	fn to_array(&self) -> Vec<u8> {
		self.try_to_array().unwrap_or_else(|err| {
			tracing::warn!(
				error = ?err,
				"Failed to serialize witness rule via safe path; falling back to legacy encoder"
			);
			let mut writer = Encoder::new();
			writer.write_u8(self.action as u8);
			writer.write_serializable_fixed(&self.condition);
			writer.to_bytes()
		})
	}
}

#[cfg(test)]
mod tests {
	use primitive_types::H160;

	use crate::{
		builder::{TransactionError, WitnessAction, WitnessCondition, WitnessRule},
		codec::{Encoder, NeoSerializable},
		config::{NeoConstants, TestConstants},
		crypto::Secp256r1PublicKey,
		neo_types::ScriptHashExtension,
	};

	#[test]
	fn test_decode_boolean_condition() {
		let json = r#"{"action": "Allow","condition": {"type": "Boolean","expression": "false"}}"#;
		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with Boolean condition");
		assert!(matches!(rule.condition, WitnessCondition::Boolean(_)));
		assert!(
			!rule
				.condition
				.boolean_expression()
				.expect("boolean_expression should return Some for Boolean condition")
		);
	}

	#[test]
	fn test_script_hash_condition_serialize_deserialize() {
		let hash = TestConstants::DEFAULT_ACCOUNT_SCRIPT_HASH;
		let condition = WitnessCondition::ScriptHash(
			H160::from_hex(hash).expect("Failed to decode DEFAULT_ACCOUNT_SCRIPT_HASH hex"),
		);

		let bytes = hex::decode(format!("18{}", hash))
			.expect("Failed to decode hex bytes for ScriptHash condition");

		let deserialized = WitnessCondition::from_bytes(&bytes)
			.expect("Failed to deserialize WitnessCondition from bytes");
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);
		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_decode_not_condition() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "Not",
            "expression": {
                "type": "Not",
                "expression": {
                    "type": "CalledByEntry"
                }
            }
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with nested Not condition");

		assert!(matches!(
			rule.condition,
			WitnessCondition::Not(boxed) if matches!(*boxed, WitnessCondition::Not(_))
		));
	}

	#[test]
	fn test_and_condition_serialize_deserialize() {
		let condition = WitnessCondition::And(vec![
			WitnessCondition::Boolean(true),
			WitnessCondition::Boolean(false),
		]);

		let bytes =
			hex::decode("020200010000").expect("Failed to decode hex bytes for And condition");

		let deserialized = WitnessCondition::from_bytes(&bytes).unwrap();
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);

		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_not_condition_serialize_deserialize() {
		let condition = WitnessCondition::Not(Box::new(WitnessCondition::CalledByEntry));

		let bytes = hex::decode("0120").expect("Failed to decode hex bytes for Not condition");

		let deserialized = WitnessCondition::from_bytes(&bytes).unwrap();
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);
		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_boolean_nil_values() {
		let json = r#"{
        "action": "Deny",
        "condition": {
            "type": "CalledByGroup",
            "group": "035a1ced7ae274a881c3f479452c8bca774c89f653d54c5c5959a01371a8c696fd"
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with CalledByGroup condition");

		assert!(rule.condition.boolean_expression().is_none());
		assert!(rule.condition.expression().is_none());
	}

	#[test]
	fn test_group_condition_invalid_key_rejected() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "Group",
            "group": "deadbeef"
        }
    }"#;

		let result: Result<WitnessRule, _> = serde_json::from_str(json);
		assert!(result.is_err());
	}

	#[test]
	fn test_decode_or_condition() {
		let json = r#"{
        "action": "Deny",
        "condition": {
            "type": "Or",
            "expressions": [
                {"type": "Group", "group": "023be7b6742268f4faca4835718f3232ddc976855d5ef273524cea36f0e8d102f3"},
                {"type": "CalledByEntry"}
            ]
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with Or condition");

		assert!(matches!(
			rule.condition,
			WitnessCondition::Or(conditions) if conditions.len() == 2
		));
	}

	#[test]
	fn test_called_by_group_condition_serialize_deserialize() {
		let key: &str = TestConstants::DEFAULT_ACCOUNT_PUBLIC_KEY;
		let condition = WitnessCondition::CalledByGroup(
			Secp256r1PublicKey::from_encoded(key)
				.expect("Failed to decode DEFAULT_ACCOUNT_PUBLIC_KEY"),
		);

		let bytes = hex::decode(format!("29{}", key))
			.expect("Failed to decode hex bytes for CalledByGroup condition");

		let deserialized = WitnessCondition::from_bytes(&bytes).unwrap();
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);

		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_called_by_entry_serialize_deserialize() {
		let condition = WitnessCondition::CalledByEntry;

		let bytes =
			hex::decode("20").expect("Failed to decode hex bytes for CalledByEntry condition");

		let deserialized = WitnessCondition::from_bytes(&bytes).unwrap();
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);

		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_called_by_contract_serialize_deserialize() {
		let hash = TestConstants::DEFAULT_ACCOUNT_SCRIPT_HASH;
		let condition = WitnessCondition::CalledByContract(
			H160::from_hex(hash).expect("Failed to decode DEFAULT_ACCOUNT_SCRIPT_HASH hex"),
		);

		let bytes = hex::decode(format!("28{}", hash))
			.expect("Failed to decode hex bytes for CalledByContract condition");

		let deserialized = WitnessCondition::from_bytes(&bytes).unwrap();
		assert_eq!(condition, deserialized);

		let mut writer = Encoder::new();
		condition.encode(&mut writer);

		assert_eq!(bytes, writer.to_bytes());
	}

	#[test]
	fn test_decode_script_hash_condition() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "ScriptHash",
            "hash": "ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with ScriptHash condition");

		assert!(matches!(rule.condition, WitnessCondition::ScriptHash(_)));
	}

	#[test]
	fn test_decode_group_condition() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "Group",
            "group": "0352321377ac7b4e1c4c2ebfe28f4d82fa3c213f7ccfcc9dac62da37fb9b433f0c"
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with Group condition");

		assert!(matches!(rule.condition, WitnessCondition::Group(_),));
	}

	#[test]
	fn test_decode_called_by_entry_condition() {
		let json = r#"{
        "action": "Deny",
        "condition": {
            "type": "CalledByEntry"
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with CalledByEntry condition");

		assert_eq!(rule.condition, WitnessCondition::CalledByEntry,);
	}

	#[test]
	fn test_decode_called_by_contract_condition() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "CalledByContract",
            "hash": "ef4073a0f2b305a38ec4050e4d3d28bc40ea63e4"
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with CalledByContract condition");

		assert!(matches!(rule.condition, WitnessCondition::CalledByContract(_),));
	}

	#[test]
	fn test_condition_try_to_array_rejects_too_many_expressions() {
		let condition = WitnessCondition::And(
			(0..=NeoConstants::MAX_SIGNER_SUBITEMS)
				.map(|_| WitnessCondition::Boolean(true))
				.collect(),
		);

		assert_eq!(condition.try_to_array(), Err(TransactionError::InvalidWitnessCondition));
	}

	#[test]
	fn test_witness_rule_try_to_array_rejects_invalid_condition() {
		let rule = WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::Or(
				(0..=NeoConstants::MAX_SIGNER_SUBITEMS)
					.map(|_| WitnessCondition::Boolean(false))
					.collect(),
			),
		);

		assert_eq!(rule.try_to_array(), Err(TransactionError::InvalidWitnessCondition));
	}

	#[test]
	fn test_and_condition_decode() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "And",
            "expressions": [
                {"type": "CalledByEntry"},
                {"type": "Group", "group": "021821807f923a3da004fb73871509d7635bcc05f41edef2a3ca5c941d8bbc1231"},
                {"type": "Boolean", "expression": "true"}
            ]
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json).expect(
			"Failed to decode WitnessRule JSON with And condition containing multiple expressions",
		);

		assert!(matches!(
			rule.condition,
			WitnessCondition::And(expressions) if expressions.len() == 3
		));
	}

	#[test]
	fn test_not_condition_decode() {
		let json = r#"{
        "action": "Allow",
        "condition": {
            "type": "Not",
            "expression": {
                "type": "CalledByEntry"
            }
        }
    }"#;

		let rule: WitnessRule = serde_json::from_str(json)
			.expect("Failed to decode WitnessRule JSON with Not condition");

		assert!(matches!(rule.condition, WitnessCondition::Not(_)));
	}

	#[test]
	fn boolean_expression() {
		let json = r#"{
        "condition": {
            "type": "Boolean",
            "expression": "false"
        }
    }"#;

		let condition = parse_condition(json);
		assert!(
			!condition
				.boolean_expression()
				.expect("boolean_expression should return Some for Boolean condition")
		);
	}

	fn parse_condition(_: &str) -> WitnessCondition {
		WitnessCondition::Boolean(false)
	}
}
