/// This example demonstrates the concept of working with smart contract ABIs in Neo N3.
///
/// An ABI (Application Binary Interface) is a specification that defines how to
/// interact with a smart contract on the blockchain. It describes the contract's
/// methods, parameters, and return types.
///
/// In a real application with Neo N3, you would:
/// 1. Define or obtain the contract's interface (methods, parameters, return types)
/// 2. Generate code to interact with the contract
/// 3. Use the generated code to make contract calls
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Neo N3 Smart Contract ABI Example");
	println!("================================");

	// 1. In a real application, you'd define your contract interface
	println!("\n1. Define your contract interface");
	println!("   This could be from a manifest.json file or defined in code");
	println!("   Example of a contract interface for a NEP-17 token:");
	println!("   {{");
	println!("     \"methods\": [");
	println!("       {{");
	println!("         \"name\": \"symbol\",");
	println!("         \"parameters\": [],");
	println!("         \"returntype\": \"String\",");
	println!("         \"offset\": 0");
	println!("       }},");
	println!("       {{");
	println!("         \"name\": \"decimals\",");
	println!("         \"parameters\": [],");
	println!("         \"returntype\": \"Integer\",");
	println!("         \"offset\": 0");
	println!("       }},");
	println!("       {{");
	println!("         \"name\": \"balanceOf\",");
	println!("         \"parameters\": [");
	println!("           {{");
	println!("             \"name\": \"account\",");
	println!("             \"type\": \"Hash160\"");
	println!("           }}");
	println!("         ],");
	println!("         \"returntype\": \"Integer\",");
	println!("         \"offset\": 0");
	println!("       }},");
	println!("       {{");
	println!("         \"name\": \"transfer\",");
	println!("         \"parameters\": [");
	println!("           {{");
	println!("             \"name\": \"from\",");
	println!("             \"type\": \"Hash160\"");
	println!("           }},");
	println!("           {{");
	println!("             \"name\": \"to\",");
	println!("             \"type\": \"Hash160\"");
	println!("           }},");
	println!("           {{");
	println!("             \"name\": \"amount\",");
	println!("             \"type\": \"Integer\"");
	println!("           }}");
	println!("         ],");
	println!("         \"returntype\": \"Boolean\",");
	println!("         \"offset\": 0");
	println!("       }}");
	println!("     ]");
	println!("   }}");

	// 2. In a real application, you would generate code from the interface
	println!("\n2. Generate code to interact with the contract");
	println!("   In Neo N3, this would be done using SmartContract implementations");
	println!("   Example of generated code for a NEP-17 token contract:");
	println!("   ```");
	println!("   impl NEP17Contract {{");
	println!("       pub async fn symbol(&self) -> Result<String, ContractError> {{ ... }}");
	println!("       pub async fn decimals(&self) -> Result<u8, ContractError> {{ ... }}");
	println!(
		"       pub async fn balance_of(&self, account: &Address) -> Result<u64, ContractError> {{ ... }}"
	);
	println!(
		"       pub async fn transfer(&self, from: &Address, to: &Address, amount: u64) -> Result<bool, ContractError> {{ ... }}"
	);
	println!("   }}");
	println!("   ```");

	// 3. In a real application, you would use the generated code
	println!("\n3. Use the generated code to interact with the contract");
	println!("   Example:");
	println!("   ```");
	println!("   let contract = NEP17Contract::new(contract_hash, provider);");
	println!("   let symbol = contract.symbol().await?;");
	println!("   let decimals = contract.decimals().await?;");
	println!("   let balance = contract.balance_of(&my_address).await?;");
	println!("   ```");

	println!("\nFor more details on Neo N3 smart contracts, refer to the Neo N3 documentation:");
	println!("https://docs.neo.org/docs/en-us/develop/write/basics.html");

	println!("\nSmart Contract ABI example completed!");
	Ok(())
}
