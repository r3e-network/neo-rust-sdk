use crate::{
	builder::{BuilderError, InvocationScript, ScriptBuilder, VerificationScript},
	codec::{Decoder, Encoder, NeoSerializable},
	crypto::{KeyPair, Secp256r1PublicKey, Secp256r1Signature},
	Bytes, ContractParameter,
};
use serde::{Deserialize, Serialize};

#[derive(Hash, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Witness {
	pub invocation: InvocationScript,
	pub verification: VerificationScript,
}

impl Default for Witness {
	fn default() -> Self {
		Self::new()
	}
}

impl Witness {
	pub fn new() -> Self {
		Self { invocation: InvocationScript::new(), verification: VerificationScript::new() }
	}

	pub fn from_scripts(invocation_script: Bytes, verification_script: Bytes) -> Self {
		Self {
			invocation: InvocationScript::new_with_script(invocation_script),
			verification: VerificationScript::from(verification_script),
		}
	}

	pub fn from_scripts_obj(
		invocation_script: InvocationScript,
		verification_script: VerificationScript,
	) -> Self {
		Self { invocation: invocation_script, verification: verification_script }
	}

	pub fn create(message_to_sign: Bytes, key_pair: &KeyPair) -> Result<Self, BuilderError> {
		let invocation_script =
			InvocationScript::from_message_and_key_pair(message_to_sign, key_pair).unwrap();
		let verification_script = VerificationScript::from_public_key(&key_pair.public_key());
		Ok(Self { invocation: invocation_script, verification: verification_script })
	}

	pub fn create_multi_sig_witness(
		signing_threshold: u8,
		signatures: Vec<Secp256r1Signature>,
		mut public_keys: Vec<Secp256r1PublicKey>,
	) -> Result<Self, BuilderError> {
		let verification_script =
			VerificationScript::from_multi_sig(public_keys.as_mut_slice(), signing_threshold);
		Self::create_multi_sig_witness_script(signatures, verification_script)
	}

	pub fn create_multi_sig_witness_script(
		signatures: Vec<Secp256r1Signature>,
		verification_script: VerificationScript,
	) -> Result<Self, BuilderError> {
		let threshold = verification_script.get_signing_threshold().unwrap();
		if signatures.len() < threshold {
			return Err(BuilderError::SignerConfiguration(
				"Not enough signatures provided for the required signing threshold.".to_string(),
			));
		}

		let invocation_script = InvocationScript::from_signatures(&signatures[..threshold]);
		Ok(Self { invocation: invocation_script, verification: verification_script })
	}

	pub fn create_contract_witness(params: Vec<ContractParameter>) -> Result<Self, BuilderError> {
		if params.is_empty() {
			return Ok(Self::new());
		}

		let mut builder = ScriptBuilder::new();
		for param in params {
			builder.push_param(&param).map_err(|e| {
				BuilderError::IllegalArgument(format!("Failed to push parameter: {}", e))
			})?;
		}
		let invocation_script = builder.to_bytes();

		Ok(Self {
			invocation: InvocationScript::new_with_script(invocation_script),
			verification: VerificationScript::new(),
		})
	}
}

impl NeoSerializable for Witness {
	type Error = BuilderError;

	fn size(&self) -> usize {
		self.invocation.size() + self.verification.size()
	}

	fn encode(&self, writer: &mut Encoder) {
		self.invocation.encode(writer);
		self.verification.encode(writer);
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error> {
		let invocation = InvocationScript::decode(reader)?;
		let verification = VerificationScript::decode(reader)?;
		Ok(Self { invocation, verification })
	}
	fn to_array(&self) -> Vec<u8> {
		let mut writer = Encoder::new();
		self.encode(&mut writer);
		writer.to_bytes()
	}
}
