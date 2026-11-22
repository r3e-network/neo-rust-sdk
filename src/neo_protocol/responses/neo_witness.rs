use crate::{Base64Encode, builder::Witness};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct NeoWitness {
	pub invocation: String,
	pub verification: String,
}

impl NeoWitness {
	pub fn new(invocation: String, verification: String) -> Self {
		Self { invocation, verification }
	}

	pub fn from_witness(witness: Witness) -> Self {
		Self {
			invocation: Base64Encode::to_base64(witness.invocation.script()),
			verification: Base64Encode::to_base64(witness.verification.script()),
		}
	}
}
