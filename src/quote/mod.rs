
use core::slice;
use serde::{Deserialize, Serialize};

use x509_parser::{der_parser::asn1_rs::{Boolean, Enumerated}, prelude::*};
use oid_registry::asn1_rs;
use asn1_rs::{oid, Sequence, FromDer, Oid, Integer, OctetString};

use p256::ecdsa::{VerifyingKey, signature::Verifier, Signature};


// https://download.01.org/intel-sgx/latest/dcap-latest/linux/docs/Intel_SGX_ECDSA_QuoteLibReference_DCAP_API.pdf

// high level sgx quote structure
// [48 - header] [384 - isv enclave report] [4 - quote signature length] [var - quote signature] 



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SgxQuote {
    pub header: SgxQuoteHeader,                 // [48 bytes]
                                                // Header of Quote data structure. This field is transparent (the user knows
                                                // its internal structure). Rest of the Quote data structure can be
                                                // treated as opaque (hidden from the user).
    pub isv_enclave_report: SgxEnclaveReport,   // [384 bytes]
                                                // Report of the attested ISV Enclave.
                                                // The CPUSVN and ISVSVN is the TCB when the quote is generated.
                                                // The REPORT.ReportData is defined by the ISV but should provide quote replay 
                                                // protection if required.
    pub signature_len: u32,                     // [4 bytes]
                                                // Size of the Quote Signature Data structure in bytes.
    pub signature: *mut u8,                     // [variable bytes]
                                                // Variable-length data containing the signature and supporting data. 
                                                // E.g. ECDSA 256-bit Quote Signature Data Structure (SgxQuoteSignatureData)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SgxQuoteHeader {
    pub version: u16,           // [2 bytes]
                                // version of the quote data structure - 3
    pub att_key_type: u16,      // [2 bytes] 
                                // Type of the Attestation Key used by the Quoting Enclave - 2 (ECDSA-256-with-P-256 curve)
    pub reserved: [u8; 4],      // [4 bytes] 
                                // Reserved for future use - 0
    pub qe_svn: u16,            // [2 bytes]
                                // Security Version of the Quoting Enclave - 1
    pub pce_svn: u16,           // [2 bytes] 
                                // Security Version of the PCE - 0
    pub qe_vendor_id: [u8; 16], // [16 bytes] 
                                // Unique identifier of the QE Vendor. 
                                // Value: 939A7233F79C4CA9940A0DB3957F0607 (Intel® SGX QE Vendor)
    pub user_data: [u8; 20],    // [20 bytes] 
                                // Custom user-defined data. 
                                // For the Intel® SGX DCAP library, the first 16 bytes contain a QE identifier that is 
                                // used to link a PCK Cert to an Enc(PPID). This identifier is consistent for
                                // every quote generated with this QE on this platform.
    
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SgxEnclaveReport {
    pub cpu_svn: [u8; 16],      // [16 bytes]
                                // Security Version of the CPU (raw value)
    pub misc_select: [u8; 4],   // [4 bytes]
                                // SSA Frame extended feature set. 
                                // Reports what SECS.MISCSELECT settings are used in the enclave. You can limit the
                                // allowed MISCSELECT settings in the sigstruct using MISCSELECT/MISCMASK.
    pub reserved_1: [u8; 28],   // [28 bytes]
                                // Reserved for future use - 0
    pub attributes: [u8; 16],   // [16 bytes]
                                // Set of flags describing attributes of the enclave.
                                // Reports what SECS.ATTRIBUTES settings are used in the enclave. The ISV can limit what
                                // SECS.ATTRIBUTES can be used when loading the enclave through parameters to the SGX Signtool.
                                // The Signtool will produce a SIGSTRUCT with ATTRIBUTES and ATTRIBUTESMASK 
                                // which determine allowed ATTRIBUTES.
                                // - For each SIGSTRUCT.ATTRIBUTESMASK bit that is set, then corresponding bit in the
                                // SECS.ATTRIBUTES must match the same bit in SIGSTRUCT.ATTRIBUTES.
    pub mrenclave: [u8; 32],    // [32 bytes] 
                                // Measurement of the enclave. 
                                // The MRENCLAVE value is the SHA256 hash of the ENCLAVEHASH field in the SIGSTRUCT.
    pub reserved_2: [u8; 32],   // [32 bytes] 
                                // Reserved for future use - 0
    pub mrsigner: [u8; 32],     // [32 bytes]
                                // Measurement of the enclave signer. 
                                // The MRSIGNER value is the SHA256 hash of the MODULUS field in the SIGSTRUCT.
    pub reserved_3: [u8; 96],   // [96 bytes]
                                // Reserved for future use - 0
    pub isv_prod_id: u16,       // [2 bytes]
                                // Product ID of the enclave. 
                                // The ISV should configure a unique ISVProdID for each product which may
                                // want to share sealed data between enclaves signed with a specific MRSIGNER. The ISV
                                // may want to supply different data to identical enclaves signed for different products.
    pub isv_svn: u16,           // [2 bytes]
                                // Security Version of the enclave
    pub reserved_4: [u8; 60],   // [60 bytes]
                                // Reserved for future use - 0
    pub report_data: [u8; 64],  // [64 bytes]
                                // Additional report data.
                                // The enclave is free to provide 64 bytes of custom data to the REPORT.
                                // This can be used to provide specific data from the enclave or it can be used to hold 
                                // a hash of a larger block of data which is provided with the quote. 
                                // The verification of the quote signature confirms the integrity of the
                                // report data (and the rest of the REPORT body).
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SgxQuoteSignatureData {
    pub isv_enclave_report_signature: [u8; 64],     // ECDSA signature, the r component followed by the s component, 2 x 32 bytes.
    pub ecdsa_attestation_key: [u8; 64],            // EC KT-I Public Key, the x-coordinate followed by the y-coordinate 
                                                    // (on the RFC 6090 P-256 curve), 2 x 32 bytes.
    pub qe_report: SgxEnclaveReport,
    pub qe_report_signature: [u8; 64],
    pub qe_auth_data: SgxQeAuthData,
    pub qe_cert_data: SgxQeCertData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SgxQeAuthData {
    pub size: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SgxQeCertData {
    pub cert_data_type: u16,
    pub cert_data_size: u32,
    pub cert_data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EnclaveReportArgs {
    // for the description of each parameter, check out SgxEnclaveReport
    pub cpu_svn: Option<[u8; 16]>,      // [16 bytes]
    pub misc_select: Option<[u8; 4]>,   // [4 bytes]
    pub attributes: Option<[u8; 16]>,   // [16 bytes]
    pub mrenclave: Option<[u8; 32]>,    // [32 bytes] 
    pub mrsigner: Option<[u8; 32]>,     // [32 bytes]
    pub isv_prod_id: Option<u16>,       // [2 bytes]
    pub isv_svn: Option<u16>,           // [2 bytes]
    pub report_data: Option<[u8; 64]>,  // [64 bytes]
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcbInfoRoot {
    pub tcb_info: TcbInfo,
    pub signature: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcbInfo {
    pub version: i64,
    pub issue_date: String,
    pub next_update: String,
    pub fmspc: String,
    pub pce_id: String,
    pub tcb_type: i64,
    pub tcb_evaluation_data_number: i64,
    pub tcb_levels: Vec<TcbLevel>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcbLevel {
    pub tcb: Tcb,
    pub tcb_date: String,
    pub tcb_status: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tcb {
    pub sgxtcbcomp01svn: u64,
    pub sgxtcbcomp02svn: u64,
    pub sgxtcbcomp03svn: u64,
    pub sgxtcbcomp04svn: u64,
    pub sgxtcbcomp05svn: u64,
    pub sgxtcbcomp06svn: u64,
    pub sgxtcbcomp07svn: u64,
    pub sgxtcbcomp08svn: u64,
    pub sgxtcbcomp09svn: u64,
    pub sgxtcbcomp10svn: u64,
    pub sgxtcbcomp11svn: u64,
    pub sgxtcbcomp12svn: u64,
    pub sgxtcbcomp13svn: u64,
    pub sgxtcbcomp14svn: u64,
    pub sgxtcbcomp15svn: u64,
    pub sgxtcbcomp16svn: u64,
    pub pcesvn: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TcbStatus {
    OK,
    TcbSwHardeningNeeded,
    TcbConfigurationAndSwHardeningNeeded,
    TcbConfigurationNeeded,
    TcbOutOfDate,
    TcbOutOfDateConfigurationNeeded,
    TcbRevoked,
    TcbUnrecognized
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcbExtension {
    pub sgxtcbcomp01svn: u64,
    pub sgxtcbcomp02svn: u64,
    pub sgxtcbcomp03svn: u64,
    pub sgxtcbcomp04svn: u64,
    pub sgxtcbcomp05svn: u64,
    pub sgxtcbcomp06svn: u64,
    pub sgxtcbcomp07svn: u64,
    pub sgxtcbcomp08svn: u64,
    pub sgxtcbcomp09svn: u64,
    pub sgxtcbcomp10svn: u64,
    pub sgxtcbcomp11svn: u64,
    pub sgxtcbcomp12svn: u64,
    pub sgxtcbcomp13svn: u64,
    pub sgxtcbcomp14svn: u64,
    pub sgxtcbcomp15svn: u64,
    pub sgxtcbcomp16svn: u64,
    pub pcesvn: u64,
    pub cpusvn: [u8; 16],
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SgxExtensions {
    pub ppid: [u8; 16],
    pub tcb: TcbExtension,
    pub pceid: [u8; 2],
    pub fmspc: [u8; 6],
    pub sgx_type: u32,
    pub platform_instance_id: Option<[u8; 16]>,
    pub configuration: Option<PckPlatformConfiguration>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PckPlatformConfiguration {
    pub dynamic_platform: Option<bool>,
    pub cached_keys: Option<bool>,
    pub smt_enabled: Option<bool>,
}

impl EnclaveReportArgs {
    pub fn default() -> EnclaveReportArgs {
        EnclaveReportArgs {
            cpu_svn: None,
            misc_select: None,
            attributes: None,
            mrenclave: None,
            mrsigner: None,
            isv_prod_id: None,
            isv_svn: None,
            report_data: None,
        }
    }
}

impl SgxQuoteSignatureData {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxQuoteSignatureData {
        let mut isv_enclave_report_signature = [0u8; 64];
        let mut ecdsa_attestation_key = [0u8; 64];
        let mut qe_report_signature = [0u8; 64];

        isv_enclave_report_signature.copy_from_slice(&raw_bytes[0..64]);
        ecdsa_attestation_key.copy_from_slice(&raw_bytes[64..128]);
        let qe_report = SgxEnclaveReport::from_bytes(&raw_bytes[128..512]);
        qe_report_signature.copy_from_slice(&raw_bytes[512..576]);
        let qe_auth_data = SgxQeAuthData::from_bytes(&raw_bytes[576..]);
        let qe_cert_data_start = 576 + 2 + qe_auth_data.size as usize;
        let qe_cert_data = SgxQeCertData::from_bytes(&raw_bytes[qe_cert_data_start..]);

        SgxQuoteSignatureData {
            isv_enclave_report_signature,
            ecdsa_attestation_key,
            qe_report,
            qe_report_signature,
            qe_auth_data,
            qe_cert_data,
        }
    }
}

impl SgxQeAuthData {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxQeAuthData {
        let size = u16::from_le_bytes([raw_bytes[0], raw_bytes[1]]);
        let data = raw_bytes[2..2+size as usize].to_vec();
        SgxQeAuthData {
            size,
            data,
        }
    }
}

impl SgxQeCertData {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxQeCertData {
        let cert_data_type = u16::from_le_bytes([raw_bytes[0], raw_bytes[1]]);
        let cert_data_size = u32::from_le_bytes([raw_bytes[2], raw_bytes[3], raw_bytes[4], raw_bytes[5]]);
        let cert_data = raw_bytes[6..6+cert_data_size as usize].to_vec();
        SgxQeCertData {
            cert_data_type,
            cert_data_size,
            cert_data
        }
    }
}

impl SgxQuote {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxQuote {
        let header = SgxQuoteHeader::from_bytes(&raw_bytes[0..48]);
        let isv_enclave_report = SgxEnclaveReport::from_bytes(&raw_bytes[48..432]);
        let signature_len = u32::from_le_bytes([raw_bytes[432], raw_bytes[433], raw_bytes[434], raw_bytes[435]]);
        // allocate and create a buffer for signature
        let signature_slice = &raw_bytes[436..];
        assert_eq!(signature_slice.len(), signature_len as usize);
        let signature = signature_slice.to_vec().into_boxed_slice().as_mut_ptr();

        SgxQuote {
            header,
            isv_enclave_report,
            signature_len,
            signature,
        }
    }
}

impl SgxQuoteHeader {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxQuoteHeader {
        assert_eq!(raw_bytes.len(), 48);

        let version = u16::from_le_bytes([raw_bytes[0], raw_bytes[1]]);
        let att_key_type = u16::from_le_bytes([raw_bytes[2], raw_bytes[3]]);
        let mut reserved = [0; 4];
        reserved.copy_from_slice(&raw_bytes[4..8]);
        let qe_svn = u16::from_le_bytes([raw_bytes[8], raw_bytes[9]]);
        let pce_svn = u16::from_le_bytes([raw_bytes[10], raw_bytes[11]]);
        let mut qe_vendor_id = [0; 16];
        qe_vendor_id.copy_from_slice(&raw_bytes[12..28]);
        let mut user_data = [0; 20];
        user_data.copy_from_slice(&raw_bytes[28..48]);

        SgxQuoteHeader {
            version,
            att_key_type,
            reserved,
            qe_svn,
            pce_svn,
            qe_vendor_id,
            user_data,
        }
    }

    pub fn to_bytes(self) -> [u8; 48] {
        let mut raw_bytes = [0; 48];
        raw_bytes[0..2].copy_from_slice(&self.version.to_le_bytes());
        raw_bytes[2..4].copy_from_slice(&self.att_key_type.to_le_bytes());
        raw_bytes[4..8].copy_from_slice(&self.reserved);
        raw_bytes[8..10].copy_from_slice(&self.qe_svn.to_le_bytes());
        raw_bytes[10..12].copy_from_slice(&self.pce_svn.to_le_bytes());
        raw_bytes[12..28].copy_from_slice(&self.qe_vendor_id);
        raw_bytes[28..48].copy_from_slice(&self.user_data);

        raw_bytes
    }
}

impl SgxEnclaveReport {
    pub fn from_bytes(raw_bytes: &[u8]) -> SgxEnclaveReport{
        assert_eq!(raw_bytes.len(), 384);
        let mut obj = SgxEnclaveReport {
            cpu_svn: [0; 16],
            misc_select: [0; 4],
            reserved_1: [0; 28],
            attributes: [0; 16],
            mrenclave: [0; 32],
            reserved_2: [0; 32],
            mrsigner: [0; 32],
            reserved_3: [0; 96],
            isv_prod_id: 0,
            isv_svn: 0,
            reserved_4: [0; 60],
            report_data: [0; 64],
        };

        // parse raw bytes into obj
        obj.cpu_svn.copy_from_slice(&raw_bytes[0..16]);
        obj.misc_select.copy_from_slice(&raw_bytes[16..20]);
        obj.reserved_1.copy_from_slice(&raw_bytes[20..48]);
        obj.attributes.copy_from_slice(&raw_bytes[48..64]);
        obj.mrenclave.copy_from_slice(&raw_bytes[64..96]);
        obj.reserved_2.copy_from_slice(&raw_bytes[96..128]);
        obj.mrsigner.copy_from_slice(&raw_bytes[128..160]);
        obj.reserved_3.copy_from_slice(&raw_bytes[160..256]);
        obj.isv_prod_id = u16::from_le_bytes([raw_bytes[256], raw_bytes[257]]);
        obj.isv_svn = u16::from_le_bytes([raw_bytes[258], raw_bytes[259]]);
        obj.reserved_4.copy_from_slice(&raw_bytes[260..320]);
        obj.report_data.copy_from_slice(&raw_bytes[320..384]);

        return obj;
    }

    pub fn to_bytes(self) -> [u8; 384] {
        // convert the struct into raw bytes
        let mut raw_bytes = [0; 384];
        // copy the fields into the raw bytes
        raw_bytes[0..16].copy_from_slice(&self.cpu_svn);
        raw_bytes[16..20].copy_from_slice(&self.misc_select);
        raw_bytes[20..48].copy_from_slice(&self.reserved_1);
        raw_bytes[48..64].copy_from_slice(&self.attributes);
        raw_bytes[64..96].copy_from_slice(&self.mrenclave);
        raw_bytes[96..128].copy_from_slice(&self.reserved_2);
        raw_bytes[128..160].copy_from_slice(&self.mrsigner);
        raw_bytes[160..256].copy_from_slice(&self.reserved_3);
        raw_bytes[256..258].copy_from_slice(&self.isv_prod_id.to_le_bytes());
        raw_bytes[258..260].copy_from_slice(&self.isv_svn.to_le_bytes());
        raw_bytes[260..320].copy_from_slice(&self.reserved_4);
        raw_bytes[320..384].copy_from_slice(&self.report_data);

        raw_bytes
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedOutput {
    pub tcb_status: TcbStatus,
    pub mr_enclave: [u8; 32],
    pub mr_signer: [u8; 32],
}

pub fn parse_pem(raw_bytes: &[u8]) -> Result<Vec<Pem>, PEMError> {
    Pem::iter_from_buffer(raw_bytes).collect()
}

pub fn parse_certchain<'a>(pem_certs: &'a[Pem]) -> Vec<X509Certificate<'a>> {
    pem_certs.iter().map(|pem| {
        pem.parse_x509().unwrap()
    }).collect()
}

pub fn verify_certchain<'a>(certs: &'a [X509Certificate<'a>], root_cert: &X509Certificate<'a>) -> bool {
    // verify that the cert chain is valid
    let mut iter = certs.iter();
    let mut prev_cert = iter.next().unwrap();
    for cert in iter {
        // verify that the previous cert signed the current cert
        if !verify_certificate(prev_cert, cert.public_key().subject_public_key.as_ref()) {
            return false;
        }
        // verify that the current cert is valid
        if !validate_certificate(prev_cert) {
            return false;
        }
        prev_cert = cert;
    }
    // verify that the root cert signed the last cert
    verify_certificate(prev_cert, root_cert.public_key().subject_public_key.as_ref())
}

fn verify_certificate(cert: &X509Certificate, public_key_raw: &[u8]) -> bool {
    // verifies that the certificate is valid
    let signature = Signature::from_der(&cert.signature_value.as_ref()).unwrap();
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_raw).unwrap();
    verifying_key.verify(&cert.tbs_certificate.as_ref(), &signature).is_ok()
}

fn validate_certificate(cert: &X509Certificate) -> bool {
    // TODO: check that the certificate is a valid cert.
    // i.e., make sure that the cert name is correct, issued by intel,
    // has not been revoked, etc.
    // for now, we'll just return true
    true
}

pub fn verify_quote(quote: SgxQuote, tcb_info_root: TcbInfoRoot) -> VerifiedOutput {
    // we'll extract the ISV (local enclave AKA the enclave that is attesting) report from the quote 
    let isv_enclave_report = quote.isv_enclave_report;

    // check that the ISV enclave (local enclave) is valid
    // we assume that there exists known good values of the enclave's measurement
    // let isv_enclave_check_args = EnclaveReportArgs {
    //     cpu_svn: None,
    //     misc_select: None,
    //     attributes: None,
    //     mrenclave: Some(MRENCLAVE),
    //     mrsigner: Some(MRSIGNER),
    //     isv_prod_id: None,
    //     isv_svn: None,
    //     report_data: None,
    // };
    // assert!(verify_enclave_report(&isv_enclave_report, isv_enclave_check_args));

    // at this point, the isv enclave reports matches what we think it is
    // next is to verify the integrity of the report

    // check that the QE Report is correct
    // we'll first parse the signature into a ECDSA Quote signature data
    let ecdsa_quote_signature_data_slice = unsafe{slice::from_raw_parts_mut(quote.signature, quote.signature_len as usize)};
    let ecdsa_quote_signature_data =  SgxQuoteSignatureData::from_bytes(ecdsa_quote_signature_data_slice);

    // verify that the isv_enclave has been signed by the quoting enclave
    let mut data = [0; 48 + 384];
    data[..48].copy_from_slice(&quote.header.to_bytes());
    data[48..432].copy_from_slice(&isv_enclave_report.to_bytes());
    let mut pubkey = [4; 65];
    pubkey[1..65].copy_from_slice(&ecdsa_quote_signature_data.ecdsa_attestation_key);
    let isv_signature = Signature::from_bytes(&ecdsa_quote_signature_data.isv_enclave_report_signature.into()).unwrap();
    let isv_verifying_key = VerifyingKey::from_sec1_bytes(&pubkey).unwrap();
    // println!("data: {:?}", hex::encode(sign_data));
    // println!("signature: {:?}", hex::encode(isv_signature.to_bytes()));
    // println!("verifying_key: {:?}", isv_verifying_key);
    assert!(isv_verifying_key.verify(&data, &isv_signature).is_ok());

    // we'll get the certchain embedded in the ecda quote signature data
    // this can be one of 5 types
    // we'll only handle type 5 for now...
    // TODO: Add support for all other types

    assert_eq!(ecdsa_quote_signature_data.qe_cert_data.cert_data_type, 5);
    let certchain_bytes = ecdsa_quote_signature_data.qe_cert_data.cert_data;
    let certchain_pems = parse_pem(&certchain_bytes).unwrap();
    let certchain = parse_certchain(&certchain_pems);
    // verify that the cert chain is valid
    // we'll assume that the root cert is the last cert in the chain
    // TODO: Replace root cert here to be the actual root cert
    let root_cert = certchain.last().unwrap();
    assert!(verify_certchain(&certchain, root_cert));

    // get the leaf certificate
    let leaf_cert = parse_certchain(&certchain_pems)[0].clone();

    // calculate the qe_report_hash
    // let mut hasher = Sha256::new();
    let qe_report_bytes = ecdsa_quote_signature_data.qe_report.to_bytes();
    // hasher.update(&qe_report_bytes);
    // let qe_report_hash = hasher.finalize();
    // println!("qe_report_bytes:: {:?}", hex::encode(qe_report_bytes));
    // println!("qe_report_hash:: {:?}", hex::encode(qe_report_hash));

    // verify the signature of the QE report
    let qe_report_signature = ecdsa_quote_signature_data.qe_report_signature;
    let qe_report_public_key = leaf_cert.public_key().subject_public_key.as_ref();
    println!("qe_pubkey: {:?}", hex::encode(qe_report_public_key));
    let qe_report_signature = Signature::from_bytes(&qe_report_signature.into()).unwrap();
    let qe_report_verifying_key = VerifyingKey::from_sec1_bytes(qe_report_public_key).unwrap();
    // println!("qe_report_signautre_bytes:: {:?}", hex::encode(qe_report_signature.to_bytes()));
    // println!("qe_report_signature:::: {:?}", qe_report_signature);
    // println!("qe_report_verifying_key:::: {:?}", qe_report_verifying_key);
    assert!(qe_report_verifying_key.verify(&qe_report_bytes, &qe_report_signature).is_ok());

    // at this point in time, we have verified everything is kosher
    // isv_enclae is signed by the qe enclave
    // qe enclave is signed by intel

    // we'll create the VerifiedOutput struct that will be produced by this function
    // this allows anyone to perform application specific checks on information such as
    // mrenclave, mrsigner, tcbstatus, etc.

    // extract the sgx extensions from the leaf certificate
    let sgx_extensions = extract_sgx_extension(&leaf_cert);
    let tcb_status = get_tcb_status(sgx_extensions, tcb_info_root);

    VerifiedOutput {
        tcb_status,
        mr_enclave: isv_enclave_report.mrenclave,
        mr_signer: isv_enclave_report.mrsigner,
    }
}

pub fn get_tcb_status(sgx_extensions: SgxExtensions, tcb_info_root: TcbInfoRoot) -> TcbStatus {
    // we'll make sure the tcbinforoot is valid
    // check that fmspc is valid
    // check that pceid is valid

    // convert fmspc and pceid to string for comparison
    assert!(hex::encode(sgx_extensions.fmspc) == tcb_info_root.tcb_info.fmspc);
    assert!(hex::encode(sgx_extensions.pceid) == tcb_info_root.tcb_info.pce_id);
    
    // now that we are sure that fmspc and pceid is the same, we'll iterate through and find the tcbstatus
    // we assume that the tcb_levels are sorted in descending svn order
    println!("sgx_extensions tcb: {:?}", sgx_extensions.tcb);
    for tcb_level in tcb_info_root.tcb_info.tcb_levels.iter() {
        let tcb = &tcb_level.tcb;
        println!("tcb: {:?}", tcb);
        if tcb.sgxtcbcomp01svn < sgx_extensions.tcb.sgxtcbcomp01svn &&
            tcb.sgxtcbcomp02svn < sgx_extensions.tcb.sgxtcbcomp02svn &&
            tcb.sgxtcbcomp03svn < sgx_extensions.tcb.sgxtcbcomp03svn &&
            tcb.sgxtcbcomp04svn < sgx_extensions.tcb.sgxtcbcomp04svn &&
            tcb.sgxtcbcomp05svn < sgx_extensions.tcb.sgxtcbcomp05svn &&
            tcb.sgxtcbcomp06svn < sgx_extensions.tcb.sgxtcbcomp06svn &&
            tcb.sgxtcbcomp07svn < sgx_extensions.tcb.sgxtcbcomp07svn &&
            tcb.sgxtcbcomp08svn < sgx_extensions.tcb.sgxtcbcomp08svn &&
            tcb.sgxtcbcomp09svn < sgx_extensions.tcb.sgxtcbcomp09svn &&
            tcb.sgxtcbcomp10svn < sgx_extensions.tcb.sgxtcbcomp10svn &&
            tcb.sgxtcbcomp11svn < sgx_extensions.tcb.sgxtcbcomp11svn &&
            tcb.sgxtcbcomp12svn < sgx_extensions.tcb.sgxtcbcomp12svn &&
            tcb.sgxtcbcomp13svn < sgx_extensions.tcb.sgxtcbcomp13svn &&
            tcb.sgxtcbcomp14svn < sgx_extensions.tcb.sgxtcbcomp14svn &&
            tcb.sgxtcbcomp15svn < sgx_extensions.tcb.sgxtcbcomp15svn &&
            tcb.sgxtcbcomp16svn < sgx_extensions.tcb.sgxtcbcomp16svn &&
            tcb.pcesvn < sgx_extensions.tcb.pcesvn {
                return match tcb_level.tcb_status.as_str() {
                    "OK" => TcbStatus::OK,
                    "TCB_SW_HARDENING_NEEDED" => TcbStatus::TcbSwHardeningNeeded,
                    "TCB_CONFIGURATION_AND_SW_HARDENING_NEEDED" => TcbStatus::TcbConfigurationAndSwHardeningNeeded,
                    "TCB_CONFIGURATION_NEEDED" => TcbStatus::TcbConfigurationNeeded,
                    "TCB_OUT_OF_DATE" => TcbStatus::TcbOutOfDate,
                    "TCB_OUT_OF_DATE_CONFIGURATION_NEEDED" => TcbStatus::TcbOutOfDateConfigurationNeeded,
                    "TCB_REVOKED" => TcbStatus::TcbRevoked,
                    _ => TcbStatus::TcbUnrecognized,
                }
        }
    }
    // we went through all the tcblevels and didn't find a match
    // shouldn't happen so we'll toggle an exception
    unreachable!();
}

pub fn verify_enclave_report(enclave_report: &SgxEnclaveReport, report_args: EnclaveReportArgs) -> bool {
    let mut results = [None; 8];
    // for each of the parameters, check if the enclave report matches the expected value
    if let Some(cpu_svn) = report_args.cpu_svn {
        results[0] = Some(enclave_report.cpu_svn == cpu_svn);
    };

    if let Some(misc_select) = report_args.misc_select {
        results[1] = Some(enclave_report.misc_select == misc_select);
    };

    if let Some(attributes) = report_args.attributes {
        results[2] = Some(enclave_report.attributes == attributes);
    };

    if let Some(mrenclave) = report_args.mrenclave {
        results[3] = Some(enclave_report.mrenclave == mrenclave);
    };

    if let Some(mrsigner) = report_args.mrsigner {
        results[4] = Some(enclave_report.mrsigner == mrsigner);
    };

    if let Some(isv_prod_id) = report_args.isv_prod_id {
        results[5] = Some(enclave_report.isv_prod_id == isv_prod_id);
    };

    if let Some(isv_svn) = report_args.isv_svn {
        results[6] = Some(enclave_report.isv_svn == isv_svn);
    };

    if let Some(report_data) = report_args.report_data {
        results[7] = Some(enclave_report.report_data == report_data);
    };

    // check if all the results are true
    for result in results.iter() {
        if result.unwrap() == false {
            return false;
        }
    }

    true
}


pub fn extract_sgx_extension<'a>(cert: &'a X509Certificate<'a>) -> SgxExtensions {
    // https://download.01.org/intel-sgx/sgx-dcap/1.20/linux/docs/SGX_PCK_Certificate_CRL_Spec-1.4.pdf

    // <SGX Extensions OID>:
    //     <PPID OID>: <PPID value>
    //     <TCB OID>:
    //          <SGX TCB Comp01 SVN OID>: <SGX TCB Comp01 SVN value>
    //          <SGX TCB Comp02 SVN OID>: <SGX TCB Comp02 SVN value>
    //          …
    //          <SGX TCB Comp16 SVN OID>: <SGX TCB Comp16 SVN value>
    //          <PCESVN OID>: <PCESVN value>
    //          <CPUSVN OID>: <CPUSVN value>
    //     <PCE-ID OID>: <PCE-ID value>
    //     <FMSPC OID>: <FMSPC value>
    //     <SGX Type OID>: <SGX Type value>
    //     <PlatformInstanceID OID>: <PlatformInstanceID value>
    //     <Configuration OID>:
    //          <Dynamic Platform OID>: <Dynamic Platform flag value>
    //          <Cached Keys OID>: <Cached Keys flag value>
    //          <SMT Enabled OID>: <SMT Enabled flag value>

    // SGX Extensions       | 1.2.840.113741.1.13.1      | mandatory | ASN.1 Sequence
    // PPID                 | 1.2.840.113741.1.13.1.1    | mandatory | ASN.1 Octet String
    // TCB                  | 1.2.840.113741.1.13.1.2    | mandatory | ASN.1 Sequence
    // SGX TCB Comp01 SVN   | 1.2.840.113741.1.13.1.2.1  | mandatory | ASN.1 Integer 
    // SGX TCB Comp02 SVN   | 1.2.840.113741.1.13.1.2.2  | mandatory | ASN.1 Integer 
    // ...
    // SGX TCB Comp16 SVN   | 1.2.840.113741.1.13.1.2.16 | mandatory | ASN.1 Integer 
    // PCESVN               | 1.2.840.113741.1.13.1.2.17 | mandatory | ASN.1 Integer 
    // CPUSVN               | 1.2.840.113741.1.13.1.2.18 | mandatory | ASN.1 Integer 
    // PCE-ID               | 1.2.840.113741.1.13.1.3    | mandatory | ASN.1 Octet String
    // FMSPC                | 1.2.840.113741.1.13.1.4    | mandatory | ASN.1 Octet String
    // SGX Type             | 1.2.840.113741.1.13.1.5    | mandatory | ASN.1 Enumerated
    // Platform Instance ID | 1.2.840.113741.1.13.1.6    | optional  | ASN.1 Octet String
    // Configuration        | 1.2.840.113741.1.13.1.7    | optional  | ASN.1 Sequence
    // Dynamic Platform     | 1.2.840.113741.1.13.1.7.1  | optional  | ASN.1 Boolean
    // Cached Keys          | 1.2.840.113741.1.13.1.7.2  | optional  | ASN.1 Boolean
    // SMT Enabled          | 1.2.840.113741.1.13.1.7.3  | optional  | ASN.1 Boolean

    let sgx_extensions_bytes = cert.get_extension_unique(&oid!(1.2.840.113741.1.13.1)).unwrap().unwrap().value;

    let (_, sgx_extensions) = Sequence::from_der(sgx_extensions_bytes).unwrap();

    // we'll process the sgx extensions here...
    let mut i = sgx_extensions.content.as_ref();

    // let's define the required information to create the SgxExtensions struct
    let mut ppid = [0; 16];
    let mut tcb = TcbExtension {
        sgxtcbcomp01svn: 0,
        sgxtcbcomp02svn: 0,
        sgxtcbcomp03svn: 0,
        sgxtcbcomp04svn: 0,
        sgxtcbcomp05svn: 0,
        sgxtcbcomp06svn: 0,
        sgxtcbcomp07svn: 0,
        sgxtcbcomp08svn: 0,
        sgxtcbcomp09svn: 0,
        sgxtcbcomp10svn: 0,
        sgxtcbcomp11svn: 0,
        sgxtcbcomp12svn: 0,
        sgxtcbcomp13svn: 0,
        sgxtcbcomp14svn: 0,
        sgxtcbcomp15svn: 0,
        sgxtcbcomp16svn: 0,
        pcesvn: 0,
        cpusvn: [0; 16],
    };
    let mut pceid = [0; 2];
    let mut fmspc = [0; 6];
    let mut sgx_type = 0;
    let mut platform_instance_id: Option<[u8; 16]> = None;
    let mut configuration: Option<PckPlatformConfiguration> = None;


    while i.len() > 0 {
        let (j, current_sequence) = Sequence::from_der(i).unwrap();
        i = j;
        let (j, current_oid) = Oid::from_der(current_sequence.content.as_ref()).unwrap();
        match current_oid.to_id_string().as_str() {
            "1.2.840.113741.1.13.1.1" => {
                let (k, ppid_bytes) = OctetString::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                ppid.copy_from_slice(ppid_bytes.as_ref());
            },
            "1.2.840.113741.1.13.1.2" => {
                let (k, tcb_sequence) = Sequence::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                // iterate through from 1 - 18
                let (k, sgxtcbcomp01svn) = get_asn1_uint64(tcb_sequence.content.as_ref(), "1.2.840.113741.1.13.1.2.1");
                let (k, sgxtcbcomp02svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.2");
                let (k, sgxtcbcomp03svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.3");
                let (k, sgxtcbcomp04svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.4");
                let (k, sgxtcbcomp05svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.5");
                let (k, sgxtcbcomp06svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.6");
                let (k, sgxtcbcomp07svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.7");
                let (k, sgxtcbcomp08svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.8");
                let (k, sgxtcbcomp09svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.9");
                let (k, sgxtcbcomp10svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.10");
                let (k, sgxtcbcomp11svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.11");
                let (k, sgxtcbcomp12svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.12");
                let (k, sgxtcbcomp13svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.13");
                let (k, sgxtcbcomp14svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.14");
                let (k, sgxtcbcomp15svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.15");
                let (k, sgxtcbcomp16svn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.16");
                let (k, pcesvn) = get_asn1_uint64(k, "1.2.840.113741.1.13.1.2.17");
                let (k, cpusvn) = get_asn1_bytes(k, "1.2.840.113741.1.13.1.2.18");

                assert_eq!(k.len(), 0);
                // copy the bytes into the tcb struct
                tcb.sgxtcbcomp01svn = sgxtcbcomp01svn;
                tcb.sgxtcbcomp02svn = sgxtcbcomp02svn;
                tcb.sgxtcbcomp03svn = sgxtcbcomp03svn;
                tcb.sgxtcbcomp04svn = sgxtcbcomp04svn;
                tcb.sgxtcbcomp05svn = sgxtcbcomp05svn;
                tcb.sgxtcbcomp06svn = sgxtcbcomp06svn;
                tcb.sgxtcbcomp07svn = sgxtcbcomp07svn;
                tcb.sgxtcbcomp08svn = sgxtcbcomp08svn;
                tcb.sgxtcbcomp09svn = sgxtcbcomp09svn;
                tcb.sgxtcbcomp10svn = sgxtcbcomp10svn;
                tcb.sgxtcbcomp11svn = sgxtcbcomp11svn;
                tcb.sgxtcbcomp12svn = sgxtcbcomp12svn;
                tcb.sgxtcbcomp13svn = sgxtcbcomp13svn;
                tcb.sgxtcbcomp14svn = sgxtcbcomp14svn;
                tcb.sgxtcbcomp15svn = sgxtcbcomp15svn;
                tcb.sgxtcbcomp16svn = sgxtcbcomp16svn;
                tcb.pcesvn = pcesvn;
                tcb.cpusvn.copy_from_slice(cpusvn.as_ref());
            },
            "1.2.840.113741.1.13.1.3" => {
                let (k, pceid_bytes) = OctetString::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                pceid.copy_from_slice(pceid_bytes.as_ref());
            },
            "1.2.840.113741.1.13.1.4" => {
                let (k, fmspc_bytes) = OctetString::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                fmspc.copy_from_slice(fmspc_bytes.as_ref());
            },
            "1.2.840.113741.1.13.1.5" => {
                let (k, sgx_type_enum) = Enumerated::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                sgx_type = sgx_type_enum.0;
            },
            "1.2.840.113741.1.13.1.6" => {
                let (k, platform_instance_id_bytes) = OctetString::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                let mut temp = [0; 16];
                temp.copy_from_slice(platform_instance_id_bytes.as_ref());
                platform_instance_id = Some(temp);

            },
            "1.2.840.113741.1.13.1.7" => {
                let (k, configuration_seq) = Sequence::from_der(j).unwrap();
                assert_eq!(k.len(), 0);
                let mut configuration_temp = PckPlatformConfiguration {
                    dynamic_platform: None,
                    cached_keys: None,
                    smt_enabled: None,
                };
                // iterate through from 1 - 3, note that some of them might be optional.
                let mut k = configuration_seq.content.as_ref();
                while k.len() > 0 {
                    let (l, asn1_seq) = Sequence::from_der(k).unwrap();
                    k = l;
                    let (l, current_oid) = Oid::from_der(asn1_seq.content.as_ref()).unwrap();
                    match current_oid.to_id_string().as_str() {
                        "1.2.840.113741.1.13.1.7.1" => {
                            let (l, dynamic_platform_bool) = Boolean::from_der(l).unwrap();
                            assert_eq!(l.len(), 0);
                            configuration_temp.dynamic_platform = Some(dynamic_platform_bool.bool());
                        },
                        "1.2.840.113741.1.13.1.7.2" => {
                            let (l, cached_keys_bool) = Boolean::from_der(l).unwrap();
                            assert_eq!(l.len(), 0);
                            configuration_temp.cached_keys = Some(cached_keys_bool.bool());
                        },
                        "1.2.840.113741.1.13.1.7.3" => {
                            let (l, smt_enabled_bool) = Boolean::from_der(l).unwrap();
                            assert_eq!(l.len(), 0);
                            configuration_temp.smt_enabled = Some(smt_enabled_bool.bool());
                        },
                        _ => {
                            unreachable!("Unknown OID: {}", current_oid.to_id_string());
                        },
                    }
                }
                // done parsing...
                configuration = Some(configuration_temp);
            },
            _ => {
                unreachable!("Unknown OID: {}", current_oid.to_id_string());
            },
        }
    }

    SgxExtensions {
        ppid,
        tcb,
        pceid,
        fmspc,
        sgx_type,
        platform_instance_id,
        configuration,
    }
}

pub fn get_asn1_bool<'a>(bytes: &'a[u8], oid_str: &str) -> (&'a[u8], bool) {
    let (k, asn1_seq) = Sequence::from_der(bytes).unwrap();
    let (l, asn1_oid) = Oid::from_der(asn1_seq.content.as_ref()).unwrap();
    assert!(oid_str.eq(&asn1_oid.to_id_string()));
    let (l, asn1_bool) = Boolean::from_der(l).unwrap();
    assert_eq!(l.len(), 0);
    (k, asn1_bool.bool())
}

pub fn get_asn1_uint64<'a>(bytes: &'a[u8], oid_str: &str) -> (&'a[u8], u64) {
    let (k, asn1_seq) = Sequence::from_der(bytes).unwrap();
    let (l, asn1_oid) = Oid::from_der(asn1_seq.content.as_ref()).unwrap();
    assert!(oid_str.eq(&asn1_oid.to_id_string()));
    let (l, asn1_int) = Integer::from_der(l).unwrap();
    assert_eq!(l.len(), 0);
    (k, asn1_int.as_u64().unwrap())
}

pub fn get_asn1_bytes<'a>(bytes: &'a[u8], oid_str: &str) -> (&'a[u8], Vec<u8>) {
    let (k, asn1_seq) = Sequence::from_der(bytes).unwrap();
    let (l, asn1_oid) = Oid::from_der(asn1_seq.content.as_ref()).unwrap();
    assert!(oid_str.eq(&asn1_oid.to_id_string()));
    let (l, asn1_bytes) = OctetString::from_der(l).unwrap();
    assert_eq!(l.len(), 0);
    (k, asn1_bytes.into_cow().to_vec())
}

mod tests {

    use super::*;
    // use oid_registry::{OID_SIG_ECDSA_WITH_SHA256, OID_SIG_ECDSA_WITH_SHA384};
    // use oid_registry::asn1_rs::{Sequence, FromBer, oid, Error};

    #[test]
    fn test_certchain_parsing() {
        let certchain_bytes = hex::decode("2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d494945386a4343424a6d674177494241674956414b7750766270377a6f7a50754144646b792b6f526e356f36704d754d416f4743437147534d343942414d430a4d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675547786864475a76636d306751304578476a415942674e5642416f4d0a45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155450a4341774351304578437a414a42674e5642415954416c56544d4234584454497a4d4467794e4449784d7a557a4d6c6f5844544d774d4467794e4449784d7a557a0a4d6c6f77634445694d434147413155454177775a535735305a5777675530645949464244537942445a584a3061575a70593246305a5445614d426747413155450a43677752535735305a577767513239796347397959585270623234784644415342674e564241634d43314e68626e526849454e7359584a684d517377435159440a5651514944414a445154454c4d416b474131554542684d4356564d775754415442676371686b6a4f5051494242676771686b6a4f50514d4242774e43414154450a764b6a754b66376969723832686d2b4d5a4151452b6847643349716d53396235634e63484a754b7a5a445970626f35496a344c7a7176704f503830706f4152730a59504233594e355537704d3777644936314b66716f344944446a434341776f77487759445652306a42426777466f41556c5739647a62306234656c4153636e550a3944504f4156634c336c5177617759445652306642475177596a42676f46366758495a616148523063484d364c79396863476b7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253397a5a3367765932567964476c6d61574e6864476c76626939324d7939775932746a636d772f593245390a6347786864475a76636d306d5a57356a62325270626d63395a4756794d4230474131556444675157424251695a7667373930317a3171554d3874534c754358580a6571314c6f54414f42674e56485138424166384542414d434273417744415944565230544151482f4241497741444343416a734743537147534962345451454e0a41515343416977776767496f4d42344743697147534962345451454e41514545454358343464705036434c5154772f785543575448306b776767466c42676f710a686b69472b453042445145434d4949425654415142677371686b69472b45304244514543415149424444415142677371686b69472b45304244514543416749420a4444415142677371686b69472b4530424451454341774942417a415142677371686b69472b4530424451454342414942417a415242677371686b69472b4530420a4451454342514943415038774551594c4b6f5a496876684e41513042416759434167442f4d42414743797147534962345451454e41514948416745424d4241470a43797147534962345451454e41514949416745414d42414743797147534962345451454e4151494a416745414d42414743797147534962345451454e4151494b0a416745414d42414743797147534962345451454e4151494c416745414d42414743797147534962345451454e4151494d416745414d42414743797147534962340a5451454e4151494e416745414d42414743797147534962345451454e4151494f416745414d42414743797147534962345451454e41514950416745414d4241470a43797147534962345451454e41514951416745414d42414743797147534962345451454e415149524167454e4d42384743797147534962345451454e415149530a4242414d44414d442f2f38424141414141414141414141414d42414743697147534962345451454e41514d45416741414d42514743697147534962345451454e0a4151514542674267616741414144415042676f71686b69472b45304244514546436745424d42344743697147534962345451454e4151594545424531784169510a72743945363234433159516b497034775241594b4b6f5a496876684e41513042427a41324d42414743797147534962345451454e415163424151482f4d4241470a43797147534962345451454e41516343415145414d42414743797147534962345451454e41516344415145414d416f4743437147534d343942414d43413063410a4d45514349445a6f63514c6478362b4f2b586d4f6b766f6b654133345a617261342b6539534e5877344b68396d5876574169415479695a6e495932474f3466670a4938673342666c4e434f56446e42505270507559377274484e77335470513d3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436c6a4343416a32674177494241674956414a567658633239472b487051456e4a3150517a7a674658433935554d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484178496a41670a42674e5642414d4d47556c756447567349464e4857434251513073675547786864475a76636d306751304578476a415942674e5642416f4d45556c75644756730a49454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b474131554543417743513045780a437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741454e53422f377432316c58534f0a3243757a7078773734654a423732457944476757357258437478327456544c7136684b6b367a2b5569525a436e71523770734f766771466553786c6d546c4a6c0a65546d693257597a33714f42757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f536347724442530a42674e5648523845537a424a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b633256790a646d6c6a5a584d75615735305a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e5648513445466751556c5739640a7a62306234656c4153636e553944504f4156634c336c517744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159420a4166384341514177436759494b6f5a497a6a30454177494452774177524149675873566b6930772b6936565947573355462f32327561586530594a446a3155650a6e412b546a44316169356343494359623153416d4435786b66545670766f34556f79695359787244574c6d5552344349394e4b7966504e2b0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap();
        let certchain_pems = parse_pem(&certchain_bytes).unwrap();
        let certs = parse_certchain(&certchain_pems);
        let root_cert_bytes = hex::decode("2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a").unwrap();
        let root_cert_pem = parse_pem(&root_cert_bytes).unwrap().pop().unwrap();
        // let root_cert = root_cert_pem.parse_x509().unwrap();
        // println!("root cert: {:?}", root_cert.serial);

        // let cert = certs[0].clone();
        // println!("{:?}", cert.tbs_certificate.as_ref());
        // println!("{:?}", cert.tbs_certificate.signature.algorithm);
        // println!("{:?}", cert.tbs_certificate.signature.parameters);
        // println!("{}", cert.tbs_certificate.signature.algorithm == OID_SIG_ECDSA_WITH_SHA256);
        // println!("{}", cert.tbs_certificate.signature.algorithm == OID_SIG_ECDSA_WITH_SHA384);
        // println!("{:?}" ,cert.public_key().raw);
        // println!("{:?}" ,cert.public_key().subject_public_key.as_ref());
        // let check = verify_certchain(&certs, &root_cert);
        // println!("{}", check);
    }

    #[test]
    fn test_tcbinfo() {
        let json_str = include_str!("../../data/tcbinfo.json");
        let tcb_info_root: TcbInfoRoot = serde_json::from_str(json_str).unwrap();
        println!("{:?}", tcb_info_root);
    }

    #[test]
    fn test_verify() {
        let json_str = include_str!("../../data/tcbinfo.json");
        let tcb_info_root: TcbInfoRoot = serde_json::from_str(json_str).unwrap();
        let dcap_quote_bytes = hex::decode("03000200000000000a000f00939a7233f79c4ca9940a0db3957f0607ce48836fd48a951172fe155220a719bd0000000014140207018001000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000700000000000000dc43f8c42d8e5f52c8bbd68f426242153f0be10630ff8cca255129a3ca03d27300000000000000000000000000000000000000000000000000000000000000001cf2e52911410fbf3f199056a98d58795a559a2e800933f7fcd13d048462271c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009113b0be77ed5d0d68680ec77206b8d587ed40679b71321ccdd5405e4d54a682000000000000000000000000000000000000000000000000000000000000000044100000d6be1afe26464a3eeb6d5ade46b24ab38ad339821cacc9a923e2e3335e73fab8a89617a1ada00d4c8a0e1ff2315ede22c49a98bc83907056ce6121c1b10d2e3abe6177e039634cfbca4739ac246fda7df8c312a98f30f57b63f3c8921fce51d90a93031f97f769637be9b028e7b007a4e458d4fa717befbd81b06905082580131414020701800100000000000000000000000000000000000000000000000000000000000000000000000000000000001500000000000000070000000000000096b347a64e5a045e27369c26e6dcda51fd7c850e9b3a3a79e718f43261dee1e400000000000000000000000000000000000000000000000000000000000000008c4f5775d796503e96137f77c68a829a0056ac8ded70140b081b094490c57bff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b0f056c5355f6c770413938c2a41ed1b5c34ecb35f85fa539f16cca7a30d6da90000000000000000000000000000000000000000000000000000000000000000a67269ee80d0dfc26e9bbcf5525ec6a7bf3f4a4092e0a6dd1bf2053962dbb49c59af50a49e784ea3c7e57cd669a9c0e6bc51e04dcc5582393c6393168846cf7e2000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0500dc0d00002d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949456a544343424453674177494241674956414a5172493559365a78484836785034424941715a4e6b326e4c7a594d416f4743437147534d343942414d430a4d484578497a416842674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b0a4442464a626e526c6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e560a4241674d416b4e424d517377435159445651514745774a56557a4165467730794d7a45784d5441784e7a45334d4452614677307a4d4445784d5441784e7a45330a4d4452614d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675132567964476c6d61574e6864475578476a415942674e560a42416f4d45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b470a413155454341774351304578437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741450a77625468574d583134443163657835317875614958456771517a69636e744b7a48454a32536f31336e384a427050314a67383673764263462f7070715a554e710a68524b4642667469584c6d4f536c614955784e6e51364f434171677767674b6b4d42384741315564497751594d426141464e446f71747031312f6b75535265590a504873555a644456386c6c4e4d477747413155644877526c4d474d77596142666f463247573268306448427a4f693876595842704c6e527964584e305a57527a0a5a584a3261574e6c63793570626e526c6243356a62323076633264344c324e6c636e52705a6d6c6a5958527062323476646a517663474e7259334a7350324e680a5058427962324e6c63334e7663695a6c626d4e765a476c755a7a316b5a584977485159445652304f42425945464a57566e41395a63736f3753356b6a2f647a4a0a594f3034534175644d41344741315564447745422f775145417749477744414d42674e5648524d4241663845416a41414d4949423141594a4b6f5a496876684e0a415130424249494278544343416345774867594b4b6f5a496876684e41513042415151514d6d5867725757774c59554164456d6c766c366153444343415751470a43697147534962345451454e41514977676746554d42414743797147534962345451454e41514942416745554d42414743797147534962345451454e415149430a416745554d42414743797147534962345451454e41514944416745434d42414743797147534962345451454e41514945416745454d42414743797147534962340a5451454e41514946416745424d42454743797147534962345451454e41514947416749416744415142677371686b69472b4530424451454342774942414441510a42677371686b69472b45304244514543434149424144415142677371686b69472b45304244514543435149424144415142677371686b69472b453042445145430a436749424144415142677371686b69472b45304244514543437749424144415142677371686b69472b45304244514543444149424144415142677371686b69470a2b45304244514543445149424144415142677371686b69472b45304244514543446749424144415142677371686b69472b4530424451454344774942414441510a42677371686b69472b45304244514543454149424144415142677371686b69472b45304244514543455149424454416642677371686b69472b453042445145430a4567515146425143424147414141414141414141414141414144415142676f71686b69472b45304244514544424149414144415542676f71686b69472b4530420a44514545424159416b473668414141774477594b4b6f5a496876684e4151304242516f424144414b42676771686b6a4f5051514441674e4841444245416942560a6e584667364277466a6945474230417162424e702b4b56734a477245744f4f49666e6f365450387031414967536c4430574e39595261575968346534656835330a314637434537664964724f55414c5177757632735948513d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436d444343416a36674177494241674956414e446f71747031312f6b7553526559504873555a644456386c6c4e4d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484578497a41680a42674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b4442464a626e526c0a6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e420a4d517377435159445651514745774a56557a425a4d424d4742797147534d34394167454743437147534d34394177454841304941424c39712b4e4d7032494f670a74646c31626b2f75575a352b5447516d38614369387a373866732b664b435133642b75447a586e56544154325a68444369667949754a77764e33774e427039690a484253534d4a4d4a72424f6a6762737767626777487759445652306a42426777466f4155496d554d316c71644e496e7a6737535655723951477a6b6e427177770a556759445652306642457377535442486f45576751345a426148523063484d364c79396a5a584a3061575a70593246305a584d7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253394a626e526c62464e4857464a76623352445153356b5a584977485159445652304f42425945464e446f0a71747031312f6b7553526559504873555a644456386c6c4e4d41344741315564447745422f77514541774942426a415342674e5648524d4241663845434441470a4151482f416745414d416f4743437147534d343942414d43413067414d4555434951434a6754627456714f795a316d336a716941584d365159613672357357530a34792f4737793875494a4778647749675271507642534b7a7a516167424c517135733541373070646f6961524a387a2f3075447a344e675639316b3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap();
        let dcap_quote = SgxQuote::from_bytes(&dcap_quote_bytes);
        let verified_output = verify_quote(dcap_quote, tcb_info_root);
        println!("{:?}", verified_output);
    }

    #[test]
    fn test_extract_cert() {
        let certchain_bytes = hex::decode("2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d494945386a4343424a6d674177494241674956414b7750766270377a6f7a50754144646b792b6f526e356f36704d754d416f4743437147534d343942414d430a4d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675547786864475a76636d306751304578476a415942674e5642416f4d0a45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155450a4341774351304578437a414a42674e5642415954416c56544d4234584454497a4d4467794e4449784d7a557a4d6c6f5844544d774d4467794e4449784d7a557a0a4d6c6f77634445694d434147413155454177775a535735305a5777675530645949464244537942445a584a3061575a70593246305a5445614d426747413155450a43677752535735305a577767513239796347397959585270623234784644415342674e564241634d43314e68626e526849454e7359584a684d517377435159440a5651514944414a445154454c4d416b474131554542684d4356564d775754415442676371686b6a4f5051494242676771686b6a4f50514d4242774e43414154450a764b6a754b66376969723832686d2b4d5a4151452b6847643349716d53396235634e63484a754b7a5a445970626f35496a344c7a7176704f503830706f4152730a59504233594e355537704d3777644936314b66716f344944446a434341776f77487759445652306a42426777466f41556c5739647a62306234656c4153636e550a3944504f4156634c336c5177617759445652306642475177596a42676f46366758495a616148523063484d364c79396863476b7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253397a5a3367765932567964476c6d61574e6864476c76626939324d7939775932746a636d772f593245390a6347786864475a76636d306d5a57356a62325270626d63395a4756794d4230474131556444675157424251695a7667373930317a3171554d3874534c754358580a6571314c6f54414f42674e56485138424166384542414d434273417744415944565230544151482f4241497741444343416a734743537147534962345451454e0a41515343416977776767496f4d42344743697147534962345451454e41514545454358343464705036434c5154772f785543575448306b776767466c42676f710a686b69472b453042445145434d4949425654415142677371686b69472b45304244514543415149424444415142677371686b69472b45304244514543416749420a4444415142677371686b69472b4530424451454341774942417a415142677371686b69472b4530424451454342414942417a415242677371686b69472b4530420a4451454342514943415038774551594c4b6f5a496876684e41513042416759434167442f4d42414743797147534962345451454e41514948416745424d4241470a43797147534962345451454e41514949416745414d42414743797147534962345451454e4151494a416745414d42414743797147534962345451454e4151494b0a416745414d42414743797147534962345451454e4151494c416745414d42414743797147534962345451454e4151494d416745414d42414743797147534962340a5451454e4151494e416745414d42414743797147534962345451454e4151494f416745414d42414743797147534962345451454e41514950416745414d4241470a43797147534962345451454e41514951416745414d42414743797147534962345451454e415149524167454e4d42384743797147534962345451454e415149530a4242414d44414d442f2f38424141414141414141414141414d42414743697147534962345451454e41514d45416741414d42514743697147534962345451454e0a4151514542674267616741414144415042676f71686b69472b45304244514546436745424d42344743697147534962345451454e4151594545424531784169510a72743945363234433159516b497034775241594b4b6f5a496876684e41513042427a41324d42414743797147534962345451454e415163424151482f4d4241470a43797147534962345451454e41516343415145414d42414743797147534962345451454e41516344415145414d416f4743437147534d343942414d43413063410a4d45514349445a6f63514c6478362b4f2b586d4f6b766f6b654133345a617261342b6539534e5877344b68396d5876574169415479695a6e495932474f3466670a4938673342666c4e434f56446e42505270507559377274484e77335470513d3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436c6a4343416a32674177494241674956414a567658633239472b487051456e4a3150517a7a674658433935554d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484178496a41670a42674e5642414d4d47556c756447567349464e4857434251513073675547786864475a76636d306751304578476a415942674e5642416f4d45556c75644756730a49454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b474131554543417743513045780a437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741454e53422f377432316c58534f0a3243757a7078773734654a423732457944476757357258437478327456544c7136684b6b367a2b5569525a436e71523770734f766771466553786c6d546c4a6c0a65546d693257597a33714f42757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f536347724442530a42674e5648523845537a424a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b633256790a646d6c6a5a584d75615735305a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e5648513445466751556c5739640a7a62306234656c4153636e553944504f4156634c336c517744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159420a4166384341514177436759494b6f5a497a6a30454177494452774177524149675873566b6930772b6936565947573355462f32327561586530594a446a3155650a6e412b546a44316169356343494359623153416d4435786b66545670766f34556f79695359787244574c6d5552344349394e4b7966504e2b0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap();
        let certchain_pems = parse_pem(&certchain_bytes).unwrap();
        let certs = parse_certchain(&certchain_pems);
        let cert = certs[0].clone();
        let sgx_extensions = extract_sgx_extension(&cert);
        println!("{:?}", sgx_extensions);
        // let extensions = cert.get_extension_unique(&oid!(1.2.840.113741.1.13.1.1)).unwrap();
        // if let Some(intel_ext_data) = extensions {
        //     println!("zl: {:?}", hex::encode(intel_ext_data.value));
        //     let vv: Result<(&[u8], std::string::String), x509_parser::nom::Err<Error>> = Sequence::from_der_and_then(intel_ext_data.value, |i| {
        //         println!("zlzl: {:?}", i);
        //         let (i, a) = String::from_der(i).unwrap();
        //         // let (i, b) = Sequence::from_der(i).unwrap();
        //         Ok((i, (a)))
        //     });
        //     println!("zl: {:?}", vv);
        // }
    }
}