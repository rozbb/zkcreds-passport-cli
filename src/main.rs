mod ark_sha256;
mod issuance_checker;
mod params;
mod passport_dump;
mod passport_info;
mod sig_verif;

use issuance_checker::{IssuanceReq, PassportHashChecker};
use params::{
    ComTree, PassportComScheme, PassportComSchemeG, PredProvingKey, PredVerifyingKey, H, HG,
    MERKLE_CRH_PARAM, STATE_ID_LEN,
};
use passport_dump::PassportDump;
use passport_info::{PersonalInfo, PersonalInfoVar};
use sig_verif::load_usa_pubkey;

use zkcreds::{
    attrs::Attrs,
    pred::{prove_birth, verify_birth},
    Com,
};

use std::{
    fs::File,
    io::{self, BufRead},
};

use ark_bls12_381::Bls12_381;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Read, SerializationError, Write};
use ark_std::rand::Rng;

const LOG2_NUM_LEAVES: u32 = 31;
const LOG2_NUM_TREES: u32 = 8;
const TREE_HEIGHT: u32 = LOG2_NUM_LEAVES + 1 - LOG2_NUM_TREES;

// Sample parameters for passport validation. All passports must expire some time after TODAY, and
// be issued by ISSUING_STATE
const TODAY: u32 = 20220101u32;
const MAX_VALID_YEARS: u32 = 10u32;
const ISSUING_STATE: [u8; STATE_ID_LEN] = *b"USA";

fn load_dump() -> PassportDump {
    let file = File::open("benches/passport/michaels_passport.json").unwrap();
    serde_json::from_reader(file).unwrap()
}

fn gen_issuance_crs<R: Rng>(rng: &mut R) -> (PredProvingKey, PredVerifyingKey) {
    // Generate the hash checker circuit's CRS
    let pk = zkcreds::pred::gen_pred_crs::<
        _,
        _,
        Bls12_381,
        PersonalInfo,
        PersonalInfoVar,
        PassportComScheme,
        PassportComSchemeG,
        H,
        HG,
    >(rng, PassportHashChecker::default())
    .unwrap();

    (pk.clone(), pk.prepare_verifying_key())
}

/// With their passport, a user constructs a `PersonalInfo` struct and requests issuance
fn user_req_issuance<R: Rng>(
    rng: &mut R,
    issuance_pk: &PredProvingKey,
) -> (PersonalInfo, IssuanceReq) {
    // Load the passport and parse it into a `PersonalInfo` struct
    let dump = load_dump();
    let my_info = PersonalInfo::from_passport(rng, &dump, TODAY, MAX_VALID_YEARS);
    let attrs_com = my_info.commit();

    // Make a hash checker struct using our private data
    let hash_checker =
        PassportHashChecker::from_passport(&dump, ISSUING_STATE, TODAY, MAX_VALID_YEARS);

    // Prove the passport hash is correctly computed
    let hash_proof = prove_birth(rng, issuance_pk, hash_checker, my_info.clone()).unwrap();

    // Now put together the issuance request
    let req = IssuanceReq {
        attrs_com,
        econtent_hash: dump.econtent_hash().to_vec(),
        sig: dump.sig,
        hash_proof,
    };

    (my_info, req)
}

/// An issuer takes an issuance request and validates it
#[must_use]
fn check_issuance(birth_vk: &PredVerifyingKey, req: &IssuanceReq) -> bool {
    // Check that the hash was computed correctly and the hash's signature is correct
    let hash_checker =
        PassportHashChecker::from_issuance_req(req, ISSUING_STATE, TODAY, MAX_VALID_YEARS);
    let sig_pubkey = load_usa_pubkey();

    verify_birth(birth_vk, &req.hash_proof, &hash_checker, &req.attrs_com).unwrap()
        && sig_pubkey.verify(&req.sig, &req.econtent_hash)
}

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generates the CRS for passport issuance
    GenCrs {
        /// Path to the outputted issuance proving key
        #[clap(short, long, parse(from_os_str), value_name = "FILE")]
        proving_key: PathBuf,

        /// Path to the outputted issuance verifying key
        #[clap(short, long, parse(from_os_str), value_name = "FILE")]
        verifying_key: PathBuf,
    },

    /// Checks a base64-encoded IssuanceReq, given via STDIN. On verification success, outputs a
    /// base64-encoded credential to STDOUT and exits with exit code 0. On failure, exits with
    /// nonzero exit code.
    Issue {
        /// Path to the issuance verifying key
        #[clap(short, long, parse(from_os_str), value_name = "FILE")]
        verifying_key: PathBuf,
    },

    /// Turns a list of credentials into a sparse merkle tree. Tree is outputted in base64 to
    /// STDOUT.
    GenTree {
        /// Path to creds file. Every line should be a base64-encoded credential outputted by the
        /// issue command.
        #[clap(short, long, parse(from_os_str), value_name = "FILE")]
        creds: PathBuf,
    },
}

fn deser_from_base64<R: Read, T: CanonicalDeserialize>(r: &mut R) -> Result<T, SerializationError> {
    let b64_reader = base64::read::DecoderReader::new(r, base64::STANDARD);
    T::deserialize_unchecked(b64_reader)
}

fn ser_to_base64<W: Write, T: CanonicalSerialize>(
    val: T,
    w: &mut W,
) -> Result<(), SerializationError> {
    let b64_writer = base64::write::EncoderWriter::new(w, base64::STANDARD);
    val.serialize(b64_writer)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::GenCrs {
            proving_key,
            verifying_key,
        } => {
            // Generate the CRS
            let mut rng = rand::thread_rng();
            let (pk, vk) = gen_issuance_crs(&mut rng);

            // Write the CRS
            let mut pk_file = File::create(proving_key).expect("couldn't create proving key file");
            let mut vk_file =
                File::create(verifying_key).expect("couldn't create verifying key file");
            ser_to_base64(pk, &mut pk_file).expect("couldn't serialize proving key");
            // TODO: Get verif key serialization working
            ser_to_base64(todo!(), &mut vk_file).expect("couldn't serialize verifying key");
        }
        Command::Issue { verifying_key } => {
            // Deserialize the request and verification key
            let req = deser_from_base64::<_, IssuanceReq>(&mut io::stdin())
                .expect("request deserialization failed");
            // TODO: Get verif key deserialization working
            let vk: PredVerifyingKey = todo!();

            // Check issuance
            assert!(check_issuance(&vk, &req), "Issuance verification failed");

            // Now output just the credential
            ser_to_base64(req.attrs_com, &mut io::stdout()).expect("couldn't serialize cred");
            println!()
        }
        Command::GenTree { creds } => {
            let mut tree = ComTree::empty(MERKLE_CRH_PARAM.clone(), TREE_HEIGHT);

            // Go through each line in the creds file and add it to the tree
            let creds_file = File::open(creds).expect("couldn't open creds file");
            let line_reader = io::BufReader::new(creds_file);
            for (i, line) in line_reader.lines().enumerate() {
                let line = line.expect("couldn't read line");
                let mut line_bytes = line.as_bytes();
                let cred: Com<PassportComScheme> =
                    deser_from_base64(&mut line_bytes).expect("couldn't deserialize cred");

                // Insert the cred into the tree
                tree.insert(i as u64, &cred);
            }

            // Now serialize the tree
            ser_to_base64(tree, &mut io::stdout()).expect("couldn't serialize tree");
        }
    }
}
