use serde_derive::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize, Debug, Clone, Hash)]
// pub struct SubmitBlock {
// 	#[serde(serialize_with = "serialize_h256")]
// 	#[serde(deserialize_with = "deserialize_h256")]
// 	pub hash: H256,
// }

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct SubmitBlock(bool);
impl SubmitBlock {
	pub fn get_submit_block(&self) -> bool {
		self.0
	}
}
