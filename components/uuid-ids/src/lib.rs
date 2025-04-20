#![no_main]

use exports::componentized::services::ids::{
    Context, Error, Guest, ServiceBindingId, ServiceInstanceId,
};
use regex_lite::Regex;
use wasi::random::random::get_random_bytes;

pub(crate) struct UuidIds {}

impl UuidIds {
    pub fn generate() -> String {
        // cribbed from the uuid crate
        let bytes: [u8; 16] = get_random_bytes(16)
            .try_into()
            .expect("unexpected number of bytes");
        let src = (u128::from_be_bytes(bytes) & 0xFFFFFFFFFFFF4FFFBFFFFFFFFFFFFFFF
            | 0x40008000000000000000)
            .to_be_bytes();

        // lowercase letters
        let lut: [u8; 16] = [
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd',
            b'e', b'f',
        ];
        let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
        let mut dst = [0; 36];

        let mut group_idx = 0;
        let mut i = 0;
        while group_idx < 5 {
            let (start, end) = groups[group_idx];
            let mut j = start;
            while j < end {
                let x = src[i];
                i += 1;

                dst[j] = lut[(x >> 4) as usize];
                dst[j + 1] = lut[(x & 0x0f) as usize];
                j += 2;
            }
            if group_idx < 4 {
                dst[end] = b'-';
            }
            group_idx += 1;
        }
        String::from_utf8(dst.into_iter().collect()).unwrap()
    }

    fn validate_instance_id(instance_id: &ServiceInstanceId) -> Result<(), Error> {
        match Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0]{12}$")
            .unwrap()
            .is_match(instance_id)
        {
            false => Err(Error::from(format!(
                "expected instance-id to be a uuid, got: {}",
                instance_id
            ))),
            true => Ok(()),
        }
    }
    fn validate_binding_id(binding_id: &ServiceBindingId) -> Result<(), Error> {
        match Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .unwrap()
            .is_match(binding_id)
        {
            false => Err(Error::from(format!(
                "expected binding-id to be a uuid, got: {}",
                binding_id
            ))),
            // double check that our binding_id is not an instance_id
            true => match Self::validate_instance_id(binding_id) {
                Ok(_) => Err(Error::from(format!(
                    "expected binding-id, got an instance-id: {}",
                    binding_id
                ))),
                Err(_) => Ok(()),
            },
        }
    }
}

impl Guest for UuidIds {
    fn generate_instance_id(_ctx: Context) -> Result<ServiceInstanceId, Error> {
        let uuid = Self::generate();
        let instance_id: ServiceInstanceId = format!("{}-000000000000", uuid[0..23].to_string());
        Ok(instance_id)
    }

    fn generate_binding_id(
        _ctx: Context,
        instance_id: ServiceInstanceId,
    ) -> Result<ServiceBindingId, Error> {
        Self::validate_instance_id(&instance_id)?;

        let uuid = Self::generate();
        let binding_id: ServiceBindingId = format!(
            "{}-{}",
            instance_id[0..23].to_string(),
            uuid[24..].to_string()
        );
        Ok(binding_id)
    }

    fn lookup_instance_id(binding_id: ServiceBindingId) -> Result<ServiceInstanceId, Error> {
        Self::validate_binding_id(&binding_id)?;

        let instance_id: ServiceInstanceId =
            format!("{}-000000000000", binding_id[0..23].to_string());
        Ok(instance_id)
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "uuid-ids",
    generate_all
});

export!(UuidIds);
