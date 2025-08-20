// SGX Enclave Configuration and Management

#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_types::*;

use super::SgxError;

/// SGX Enclave configuration
#[derive(Debug, Clone)]
pub struct EnclaveConfig {
	pub heap_size: usize,
	pub stack_size: usize,
	pub tcs_num: u32,
	pub debug: bool,
	pub product_id: u16,
	pub isv_svn: u16,
	pub misc_select: u32,
	pub attributes: EnclaveAttributes,
}

impl Default for EnclaveConfig {
	fn default() -> Self {
		Self {
			heap_size: 0x1000_0000, // 256MB
			stack_size: 0x40_0000,  // 4MB
			tcs_num: 10,            // 10 threads
			debug: cfg!(debug_assertions),
			product_id: 0,
			isv_svn: 0,
			misc_select: 0,
			attributes: EnclaveAttributes::default(),
		}
	}
}

/// Enclave attributes configuration
#[derive(Debug, Clone)]
pub struct EnclaveAttributes {
	pub provision_key: bool,
	pub einit_token_key: bool,
	pub kss: bool,
}

impl Default for EnclaveAttributes {
	fn default() -> Self {
		Self { provision_key: false, einit_token_key: false, kss: false }
	}
}

/// SGX Enclave instance
pub struct SgxEnclave {
	config: EnclaveConfig,
	#[cfg(feature = "sgx")]
	enclave_id: sgx_enclave_id_t,
	initialized: bool,
}

impl SgxEnclave {
	/// Create a new enclave instance
	pub fn new(config: EnclaveConfig) -> Result<Self, SgxError> {
		Ok(Self {
			config,
			#[cfg(feature = "sgx")]
			enclave_id: 0,
			initialized: false,
		})
	}

	/// Initialize the enclave
	#[cfg(feature = "sgx")]
	pub fn initialize(&mut self) -> Result<(), SgxError> {
		if self.initialized {
			return Ok(());
		}

		// Load and initialize the enclave
		let mut launch_token = sgx_launch_token_t { body: [0; 1024] };
		let mut launch_token_updated = 0i32;
		let debug = if self.config.debug { 1 } else { 0 };

		let result = unsafe {
			sgx_create_enclave(
				b"enclave.signed.so\0".as_ptr() as *const i8,
				debug,
				&mut launch_token,
				&mut launch_token_updated,
				&mut self.enclave_id,
				std::ptr::null_mut(),
			)
		};

		if result != sgx_status_t::SGX_SUCCESS {
			return Err(SgxError::EnclaveError(format!("Failed to create enclave: {:?}", result)));
		}

		self.initialized = true;
		Ok(())
	}

	#[cfg(not(feature = "sgx"))]
	pub fn initialize(&mut self) -> Result<(), SgxError> {
		self.initialized = true;
		Ok(())
	}

	/// Check if enclave is initialized
	pub fn is_initialized(&self) -> bool {
		self.initialized
	}

	/// Get enclave configuration
	pub fn config(&self) -> &EnclaveConfig {
		&self.config
	}

	/// Execute a function inside the enclave (ECALL)
	#[cfg(feature = "sgx")]
	pub fn ecall<T, R>(&self, function_id: u32, input: &T) -> Result<R, SgxError>
	where
		T: EnclaveSerializable,
		R: EnclaveDeserializable,
	{
		if !self.initialized {
			return Err(SgxError::EnclaveError("Enclave not initialized".into()));
		}

		// Serialize input
		let input_bytes = input.serialize_for_enclave()?;

		// Perform ECALL
		let mut output_buffer = vec![0u8; R::expected_size()];
		let mut output_size = output_buffer.len();

		// Perform the actual ECALL
		let mut retval = sgx_status_t::SGX_SUCCESS;
		let result = unsafe {
			sgx_ecall(
				self.enclave_id,
				function_id as i32,
				input_bytes.as_ptr() as *const core::ffi::c_void,
				input_bytes.len(),
				output_buffer.as_mut_ptr() as *mut core::ffi::c_void,
				output_buffer.len(),
				&mut retval,
			)
		};

		if result != sgx_status_t::SGX_SUCCESS {
			return Err(SgxError::EnclaveError("ECALL failed".into()));
		}

		// Deserialize output
		R::deserialize_from_enclave(&output_buffer[..output_size])
	}

	#[cfg(not(feature = "sgx"))]
	pub fn ecall<T, R>(&self, _function_id: u32, _input: &T) -> Result<R, SgxError>
	where
		T: EnclaveSerializable,
		R: EnclaveDeserializable,
	{
		Err(SgxError::EnclaveError("SGX not enabled".into()))
	}

	/// Destroy the enclave
	#[cfg(feature = "sgx")]
	pub fn destroy(&mut self) -> Result<(), SgxError> {
		if !self.initialized {
			return Ok(());
		}

		// Destroy the enclave
		let result = unsafe { sgx_destroy_enclave(self.enclave_id) };

		if result != sgx_status_t::SGX_SUCCESS {
			return Err(SgxError::EnclaveError(format!("Failed to destroy enclave: {:?}", result)));
		}

		self.enclave_id = 0;
		self.initialized = false;

		Ok(())
	}

	#[cfg(not(feature = "sgx"))]
	pub fn destroy(&mut self) -> Result<(), SgxError> {
		self.initialized = false;
		Ok(())
	}
}

/// Trait for types that can be serialized for enclave communication
pub trait EnclaveSerializable {
	fn serialize_for_enclave(&self) -> Result<Vec<u8>, SgxError>;
}

/// Trait for types that can be deserialized from enclave communication
pub trait EnclaveDeserializable: Sized {
	fn expected_size() -> usize;
	fn deserialize_from_enclave(data: &[u8]) -> Result<Self, SgxError>;
}

// Implement for common types
impl EnclaveSerializable for Vec<u8> {
	fn serialize_for_enclave(&self) -> Result<Vec<u8>, SgxError> {
		Ok(self.clone())
	}
}

impl EnclaveDeserializable for Vec<u8> {
	fn expected_size() -> usize {
		4096 // Default buffer size
	}

	fn deserialize_from_enclave(data: &[u8]) -> Result<Self, SgxError> {
		Ok(data.to_vec())
	}
}

impl EnclaveSerializable for String {
	fn serialize_for_enclave(&self) -> Result<Vec<u8>, SgxError> {
		Ok(self.as_bytes().to_vec())
	}
}

impl EnclaveDeserializable for String {
	fn expected_size() -> usize {
		4096 // Default buffer size
	}

	fn deserialize_from_enclave(data: &[u8]) -> Result<Self, SgxError> {
		String::from_utf8(data.to_vec())
			.map_err(|e| SgxError::EnclaveError(format!("UTF-8 error: {}", e)))
	}
}

/// Generate enclave configuration file content
pub fn generate_enclave_config(config: &EnclaveConfig) -> String {
	format!(
		r#"<EnclaveConfiguration>
    <ProdID>{}</ProdID>
    <ISVSVN>{}</ISVSVN>
    <StackMaxSize>{:#x}</StackMaxSize>
    <HeapMaxSize>{:#x}</HeapMaxSize>
    <TCSNum>{}</TCSNum>
    <TCSPolicy>1</TCSPolicy>
    <DisableDebug>{}</DisableDebug>
    <MiscSelect>{:#x}</MiscSelect>
    <MiscMask>0xFFFFFFFF</MiscMask>
</EnclaveConfiguration>"#,
		config.product_id,
		config.isv_svn,
		config.stack_size,
		config.heap_size,
		config.tcs_num,
		if config.debug { 0 } else { 1 },
		config.misc_select
	)
}

// External SGX functions
#[cfg(feature = "sgx")]
extern "C" {
	fn sgx_create_enclave(
		file_name: *const i8,
		debug: i32,
		launch_token: *mut sgx_launch_token_t,
		launch_token_updated: *mut i32,
		enclave_id: *mut sgx_enclave_id_t,
		misc_attr: *mut sgx_misc_attribute_t,
	) -> sgx_status_t;

	fn sgx_destroy_enclave(enclave_id: sgx_enclave_id_t) -> sgx_status_t;

	fn sgx_ecall(
		eid: sgx_enclave_id_t,
		index: i32,
		ocall_table: *const core::ffi::c_void,
		ms: *mut core::ffi::c_void,
		ms_size: usize,
		status: *mut sgx_status_t,
	) -> sgx_status_t;
}

/// Generate Enclave Definition Language (EDL) file content
pub fn generate_edl() -> &'static str {
	r#"enclave {
    from "sgx_tstd.edl" import *;
    from "sgx_stdio.edl" import *;
    from "sgx_backtrace.edl" import *;
    from "sgx_tstdc.edl" import *;
    
    trusted {
        /* NEO blockchain operations */
        public sgx_status_t neo_sign_transaction(
            [in, size=data_len] const uint8_t* data,
            size_t data_len,
            [out, size=64] uint8_t* signature
        );
        
        public sgx_status_t neo_verify_signature(
            [in, size=data_len] const uint8_t* data,
            size_t data_len,
            [in, size=64] const uint8_t* signature,
            [in, size=64] const uint8_t* public_key,
            [out] uint8_t* valid
        );
        
        public sgx_status_t neo_calculate_hash(
            [in, size=data_len] const uint8_t* data,
            size_t data_len,
            [out, size=32] uint8_t* hash
        );
        
        public sgx_status_t neo_generate_keypair(
            [out, size=32] uint8_t* private_key,
            [out, size=64] uint8_t* public_key
        );
        
        public sgx_status_t neo_encrypt_wallet(
            [in, size=wallet_len] const uint8_t* wallet_data,
            size_t wallet_len,
            [in, size=32] const uint8_t* password_hash,
            [out, size=encrypted_len] uint8_t* encrypted_data,
            size_t encrypted_len
        );
        
        public sgx_status_t neo_decrypt_wallet(
            [in, size=encrypted_len] const uint8_t* encrypted_data,
            size_t encrypted_len,
            [in, size=32] const uint8_t* password_hash,
            [out, size=wallet_len] uint8_t* wallet_data,
            size_t wallet_len
        );
    };
    
    untrusted {
        /* OCALL for secure network communication */
        sgx_status_t ocall_neo_rpc_request(
            [in, size=request_len] const uint8_t* request,
            size_t request_len,
            [out, size=response_len] uint8_t* response,
            size_t response_len,
            [out] size_t* actual_response_len
        );
        
        /* OCALL for secure storage */
        sgx_status_t ocall_secure_save(
            [in, size=data_len] const uint8_t* data,
            size_t data_len,
            [in, size=32] const uint8_t* key_id
        );
        
        sgx_status_t ocall_secure_load(
            [in, size=32] const uint8_t* key_id,
            [out, size=data_len] uint8_t* data,
            size_t data_len,
            [out] size_t* actual_data_len
        );
    };
};"#
}
