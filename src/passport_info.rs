use crate::{
    params::{
        Fr, PassportComScheme, PassportComSchemeG, DATE_LEN, DOB_OFFSET, EXPIRY_OFFSET, HASH_LEN,
        NAME_LEN, NAME_OFFSET, NATIONALITY_OFFSET, PASSPORT_COM_PARAM, STATE_ID_LEN,
    },
    passport_dump::PassportDump,
};

use core::borrow::Borrow;

use sha2::{Digest, Sha256};
use zkcreds::{
    attrs::{AccountableAttrs, AccountableAttrsVar, Attrs, AttrsVar},
    Bytestring, ComNonce, ComNonceVar, ComParam, ComParamVar,
};

use ark_ff::{to_bytes, UniformRand};
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    bits::ToBytesGadget,
    fields::fp::FpVar,
    uint8::UInt8,
    R1CSVar,
};
use ark_relations::{
    ns,
    r1cs::{Namespace, SynthesisError},
};
use ark_std::rand::Rng;

/// Simple blob containing user's biometrics
#[derive(Clone, Default)]
pub(crate) struct Biometrics(Vec<u8>);

impl Biometrics {
    pub fn hash(&self) -> [u8; HASH_LEN] {
        Sha256::digest(&self.0).into()
    }
}

/// Stores a subset of the info found in data groups 1 and 2 of a passport
#[derive(Clone)]
pub(crate) struct PersonalInfo {
    nonce: ComNonce<PassportComScheme>,
    pub(crate) seed: Fr,
    pub(crate) nationality: [u8; STATE_ID_LEN],
    pub(crate) name: [u8; NAME_LEN],
    pub(crate) dob: u32,
    pub(crate) passport_expiry: u32,
    pub(crate) biometrics: Biometrics,
}

impl Default for PersonalInfo {
    fn default() -> PersonalInfo {
        PersonalInfo {
            nonce: ComNonce::<PassportComScheme>::default(),
            seed: Fr::default(),
            nationality: [0u8; STATE_ID_LEN],
            name: [0u8; NAME_LEN],
            dob: 0u32,
            passport_expiry: 0u32,
            biometrics: Biometrics::default(),
        }
    }
}

/// Stores a subset of the info found in data groups 1 and 2 of a passport
#[derive(Clone)]
pub(crate) struct PersonalInfoVar {
    nonce: ComNonceVar<PassportComScheme, PassportComSchemeG, Fr>,
    pub(crate) seed: FpVar<Fr>,
    pub(crate) nationality: Bytestring<Fr>,
    pub(crate) name: Bytestring<Fr>,
    pub(crate) dob: FpVar<Fr>,
    pub(crate) passport_expiry: FpVar<Fr>,
    pub(crate) biometric_hash: Bytestring<Fr>,
}

/// Converts a date string of the form YYMMDD to a u32 whose base-10 representation is YYYYMMDD.
/// `not_after` is the soonest day in the 21st century after which the input would not make sense,
/// e.g., a birthdate wouldn't make sense if it were after today, and a document expiry date
/// wouldn't be 20 years in the future.
fn date_to_u32(date: &[u8], not_after: u32) -> u32 {
    assert_eq!(date.len(), DATE_LEN);

    let century = 1000000;
    let twenty_first_century = 20 * century;

    // Converts ASCII numbers to the numbers they represent. E.g., int(b"9") = 9 (mod |Fr|)
    fn int(char: u8) -> u32 {
        (char as u32) - 48
    }

    // Convert the year, month, and day separately. b"YY" becomes YY (mod |Fr|), etc.
    let year = (int(date[0]) * 10) + int(date[1]);
    let month = (int(date[2]) * 10) + int(date[3]);
    let day = (int(date[4]) * 10) + int(date[5]);

    // Now combine the values by shifting and adding. The year is only given as YY so we don't
    // immediately have the most significant digits of the year. Assume for now that it's the 21st
    // century
    let mut d = twenty_first_century + (year * 10000) + (month * 100) + day;

    // If the date is not from the 21st century, then d exceeds the `not_after` limit. If that's
    // the case remove 100 years
    if d > not_after {
        d -= century;
    }

    d
}

impl PersonalInfo {
    /// Converts the given passport dump into a structured attribute struct. Requires `today` as an
    /// integer whose base-10 representation is of the form YYYYMMDD. `max_valid_years` is the
    /// longest that a passport can be valid, in years.
    pub fn from_passport<R: Rng>(
        rng: &mut R,
        dump: &PassportDump,
        today: u32,
        max_valid_years: u32,
    ) -> PersonalInfo {
        // Create an empty info struct that we'll fill with data
        let mut info = PersonalInfo {
            nonce: ComNonce::<PassportComScheme>::rand(rng),
            seed: Fr::rand(rng),
            ..Default::default()
        };

        // The earliest time after which expiry doesn't make sense. This is used to parse the
        // underdefined date format in the passport
        let expiry_not_after = today + max_valid_years * 10000u32;

        // Extract the nationality, name, and DOB from the DG1 blob. The biometrics are set equal
        // to the entire DG2 blob
        info.nationality
            .copy_from_slice(&dump.dg1[NATIONALITY_OFFSET..NATIONALITY_OFFSET + STATE_ID_LEN]);
        info.name
            .copy_from_slice(&dump.dg1[NAME_OFFSET..NAME_OFFSET + NAME_LEN]);
        info.dob = date_to_u32(&dump.dg1[DOB_OFFSET..DOB_OFFSET + DATE_LEN], today);
        info.passport_expiry = date_to_u32(
            &dump.dg1[EXPIRY_OFFSET..EXPIRY_OFFSET + DATE_LEN],
            expiry_not_after,
        );
        info.biometrics.0 = dump.dg2.clone();

        info
    }
}

impl Attrs<Fr, PassportComScheme> for PersonalInfo {
    /// Serializes the attrs into bytes
    fn to_bytes(&self) -> Vec<u8> {
        // DOB bytes need to match the PersonalInfoVar version, which is an FpVar. Convert to Fr
        // before serializing
        let dob = Fr::from(self.dob);
        let passport_expiry = Fr::from(self.passport_expiry);
        let biometric_hash = self.biometrics.hash();
        to_bytes![
            self.seed,
            self.nationality,
            self.name,
            dob,
            passport_expiry,
            biometric_hash
        ]
        .unwrap()
    }

    fn get_com_param(&self) -> &ComParam<PassportComScheme> {
        &*PASSPORT_COM_PARAM
    }

    fn get_com_nonce(&self) -> &ComNonce<PassportComScheme> {
        &self.nonce
    }
}

impl AccountableAttrs<Fr, PassportComScheme> for PersonalInfo {
    type Id = Vec<u8>;
    type Seed = Fr;

    fn get_id(&self) -> Vec<u8> {
        self.name.to_vec()
    }

    fn get_seed(&self) -> Fr {
        self.seed
    }
}

impl ToBytesGadget<Fr> for PersonalInfoVar {
    fn to_bytes(&self) -> Result<Vec<UInt8<Fr>>, SynthesisError> {
        Ok([
            self.seed.to_bytes()?,
            self.nationality.0.to_bytes()?,
            self.name.0.to_bytes()?,
            self.dob.to_bytes()?,
            self.passport_expiry.to_bytes()?,
            self.biometric_hash.0.to_bytes()?,
        ]
        .concat())
    }
}

impl AllocVar<PersonalInfo, Fr> for PersonalInfoVar {
    // Allocates a vector of UInt8s. This panics if `f()` is `Err`, since we don't know how many
    // bytes to allocate
    fn new_variable<T: Borrow<PersonalInfo>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into().cs();
        let native_attrs = f();

        // Make placeholder content if native_attrs is empty
        let default_info = PersonalInfo::default();

        // Unpack the given attributes
        let PersonalInfo {
            ref nonce,
            ref seed,
            ref nationality,
            ref name,
            ref dob,
            ref passport_expiry,
            ref biometrics,
        } = native_attrs
            .as_ref()
            .map(Borrow::borrow)
            .unwrap_or(&default_info);

        let biometric_hash = biometrics.hash().to_vec();

        // Witness the nonce
        let nonce = ComNonceVar::<PassportComScheme, PassportComSchemeG, Fr>::new_variable(
            ns!(cs, "nonce"),
            || Ok(nonce),
            mode,
        )?;

        // Witness all the other variables
        let seed = FpVar::<Fr>::new_variable(ns!(cs, "seed"), || Ok(seed), mode)?;
        let nationality =
            Bytestring::new_variable(ns!(cs, "nationality"), || Ok(nationality.to_vec()), mode)?;
        let name = Bytestring::new_variable(ns!(cs, "name"), || Ok(name.to_vec()), mode)?;
        let dob = FpVar::<Fr>::new_variable(ns!(cs, "dob"), || Ok(Fr::from(*dob)), mode)?;
        let passport_expiry = FpVar::<Fr>::new_variable(
            ns!(cs, "passport expiry"),
            || Ok(Fr::from(*passport_expiry)),
            mode,
        )?;
        let biometric_hash =
            Bytestring::new_variable(ns!(cs, "biometric_hash"), || Ok(biometric_hash), mode)?;

        // Return the witnessed values
        Ok(PersonalInfoVar {
            nonce,
            seed,
            nationality,
            name,
            dob,
            passport_expiry,
            biometric_hash,
        })
    }
}

impl AttrsVar<Fr, PersonalInfo, PassportComScheme, PassportComSchemeG> for PersonalInfoVar {
    fn get_com_param(
        &self,
    ) -> Result<ComParamVar<PassportComScheme, PassportComSchemeG, Fr>, SynthesisError> {
        let cs = self
            .nationality
            .cs()
            .or(self.name.cs())
            .or(self.dob.cs())
            .or(self.biometric_hash.cs());
        ComParamVar::<_, PassportComSchemeG, _>::new_constant(cs, &*PASSPORT_COM_PARAM)
    }

    fn get_com_nonce(
        &self,
    ) -> Result<ComNonceVar<PassportComScheme, PassportComSchemeG, Fr>, SynthesisError> {
        Ok(self.nonce.clone())
    }
}

impl AccountableAttrsVar<Fr, PersonalInfo, PassportComScheme, PassportComSchemeG>
    for PersonalInfoVar
{
    type Id = Bytestring<Fr>;
    type Seed = FpVar<Fr>;

    fn get_id(&self) -> Result<Bytestring<Fr>, SynthesisError> {
        Ok(self.name.clone())
    }

    fn get_seed(&self) -> Result<FpVar<Fr>, SynthesisError> {
        Ok(self.seed.clone())
    }
}
