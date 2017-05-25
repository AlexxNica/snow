#![cfg(feature = "vector-tests")]
extern crate hex;
extern crate snow;

#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use serde::de::{self, Deserialize, Deserializer, Visitor, Unexpected};
use std::ops::Deref;
use hex::{FromHex, ToHex};
use snow::*;
use snow::params::*;
use std::fmt;

struct HexBytes {
    original: String,
    payload: Vec<u8>,
}

impl Deref for HexBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl fmt::Debug for HexBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.original)
    }
}

struct HexBytesVisitor;
impl<'de> Visitor<'de> for HexBytesVisitor {
    type Value = HexBytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a hex string")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        let bytes = Vec::<u8>::from_hex(s).map_err(|_| de::Error::invalid_value(Unexpected::Str(s), &self))?;
        Ok(HexBytes {
            original: s.to_owned(),
            payload: bytes,
        })
    }

}

impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<HexBytes, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_str(HexBytesVisitor)
    }
}

#[derive(Deserialize)]
struct TestMessage {
    payload: HexBytes,
    ciphertext: HexBytes,
}

impl fmt::Debug for TestMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Message")
    }
}

#[derive(Deserialize, Debug)]
struct TestVector {
    name: String,
    init_psk: Option<HexBytes>,
    init_prologue: Option<HexBytes>,
    init_static: Option<HexBytes>,
    init_remote_static: Option<HexBytes>,
    init_ephemeral: Option<HexBytes>,
    resp_prologue: Option<HexBytes>,
    resp_static: Option<HexBytes>,
    resp_remote_static: Option<HexBytes>,
    resp_ephemeral: Option<HexBytes>,
    messages: Vec<TestMessage>,
}

#[derive(Deserialize)]
struct TestVectors {
    vectors: Vec<TestVector>,
}

fn build_session_pair(vector: &TestVector) -> Result<(Session, Session), NoiseError> {
    let params: NoiseParams = vector.name.parse().unwrap();
    let mut init_builder = NoiseBuilder::new(params.clone());
    let mut resp_builder = NoiseBuilder::new(params.clone());

    if params.handshake.is_psk() {
        match (params.handshake.modifiers.list[0], &vector.init_psk) {
            (HandshakeModifier::Psk(n), &Some(ref psk)) => {
                init_builder = init_builder.psk(n, &*psk);
                resp_builder = resp_builder.psk(n, &*psk);
            },
            _ => {
                panic!("PSK handshake using weird modifiers I don't want to deal with")
            }
        }
    }

    if let Some(ref init_s) = vector.init_static {
        init_builder = init_builder.local_private_key(&*init_s);
    }
    if let Some(ref resp_s) = vector.resp_static {
        resp_builder = resp_builder.local_private_key(&*resp_s);
    }
    if let Some(ref init_remote_static) = vector.init_remote_static {
        init_builder = init_builder.remote_public_key(&*init_remote_static);
    }
    if let Some(ref resp_remote_static) = vector.resp_remote_static {
        resp_builder = resp_builder.remote_public_key(&*resp_remote_static);
    }
    if let Some(ref init_e) = vector.init_ephemeral {
        init_builder = init_builder.fixed_ephemeral_key_for_testing_only(&*init_e);
    }
    if let Some(ref resp_e) = vector.resp_ephemeral {
        resp_builder = resp_builder.fixed_ephemeral_key_for_testing_only(&*resp_e);
    }
    if let Some(ref init_prologue) = vector.init_prologue {
        init_builder = init_builder.prologue(&*init_prologue);
    }
    if let Some(ref resp_prologue) = vector.resp_prologue {
        resp_builder = resp_builder.prologue(&*resp_prologue);
    }

    let init = init_builder.build_initiator()?;
    let resp = resp_builder.build_responder()?;

    Ok((init, resp))
}

fn confirm_message_vectors(mut init: Session, mut resp: Session, messages_vec: &Vec<TestMessage>, is_oneway: bool) -> Result<(), String> {
    let (mut sendbuf, mut recvbuf) = ([0u8; 65535], [0u8; 65535]);
    let mut messages = messages_vec.iter().enumerate();
    while !init.is_handshake_finished() {
        let (i, message) = messages.next().unwrap();
        let (send, recv) = if i % 2 == 0 {
            (&mut init, &mut resp)
        } else {
            (&mut resp, &mut init)
        };

        let len = send.write_message(&*message.payload, &mut sendbuf).map_err(|_| format!("write_message failed on message {}", i))?;
        recv.read_message(&sendbuf[..len], &mut recvbuf).map_err(|_| format!("read_message failed on message {}", i))?;
        if &sendbuf[..len] != &(*message.ciphertext)[..] {
            let mut s = String::new();
            s.push_str(&format!("message {}\n", i));
            s.push_str(&format!("plaintext: {}\n", message.payload.to_hex()));
            s.push_str(&format!("expected:  {}\n", message.ciphertext.to_hex()));
            s.push_str(&format!("actual:    {}", &sendbuf[..len].to_owned().to_hex()));
            return Err(s)
        }
    }

    let (mut init, mut resp) = (init.into_transport_mode().unwrap(), resp.into_transport_mode().unwrap());
    for (i, message) in messages {
        let (send, recv) = if is_oneway || i % 2 == 0 {
            (&mut init, &mut resp)
        } else {
            (&mut resp, &mut init)
        };

        let len = send.write_message(&*message.payload, &mut sendbuf).unwrap();
        recv.read_message(&sendbuf[..len], &mut recvbuf).unwrap();
        if &sendbuf[..len] != &(*message.ciphertext)[..] {
            let mut s = String::new();
            s.push_str(&format!("message {}", i));
            s.push_str(&format!("plaintext: {}\n", message.payload.to_hex()));
            s.push_str(&format!("expected:  {}\n", message.ciphertext.to_hex()));
            s.push_str(&format!("actual:    {}", &sendbuf[..message.ciphertext.len()].to_owned().to_hex()));
            return Err(s)
        }
    }
    Ok(())
}

fn test_vectors_from_json(json: &str) {
    let test_vectors: TestVectors = serde_json::from_str(json).unwrap();

    let mut passes = 0;
    let mut fails = 0;
    let mut ignored = 0;

    for vector in test_vectors.vectors {
        let params: NoiseParams = vector.name.parse().unwrap();
        if params.dh == DHChoice::Ed448 || params.base == BaseChoice::NoisePSK {
            ignored += 1;
            continue;
        }
        let (init, resp) = match build_session_pair(&vector) {
            Ok((init, resp)) => (init, resp),
            Err(e) => {
                println!("failure building session");
                println!("vector: {:?}", &vector);
                panic!("FAIL");
            }
        };

        match confirm_message_vectors(init, resp, &vector.messages, params.handshake.pattern.is_oneway()) {
            Ok(_) => {
                passes += 1;
            },
            Err(s) => {
                fails += 1;
                println!("FAIL");
                println!("{}", s);
                println!("{:?}", vector);
            }
        }
    }

    println!("\n{}/{} passed", passes, passes+fails);
    println!("* ignored {} unsupported variants", ignored);
    if fails > 0 {
        panic!("at least one vector failed.");
    }
}

#[test]
fn test_vectors_noise_c_basic() {
    test_vectors_from_json(include_str!("vectors/noise-c-basic.txt"));
}

#[test]
fn test_vectors_cacophony() {
    test_vectors_from_json(include_str!("vectors/cacophony.txt"));
}

#[test]
fn test_vectors_noise_go_rev32() {
    test_vectors_from_json(include_str!("vectors/noise-go.txt"));
}
