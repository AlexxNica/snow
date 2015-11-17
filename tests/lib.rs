extern crate screech;
extern crate rustc_serialize;

use screech::*;
use self::rustc_serialize::hex::{FromHex, ToHex};


struct RandomInc {
    next_byte: u8
}

impl Random for RandomInc {

    fn new() -> RandomInc {
        RandomInc {next_byte: 0}
    }

    fn fill_bytes(&mut self, out: &mut [u8]) {
        for count in 0..out.len() {
            out[count] = self.next_byte;
            if self.next_byte == 255 {
                self.next_byte = 0;
            }
            else {
                self.next_byte += 1;
            }
        }
    }
}


struct RandomSequence {
    next_bytes: [u8; 1024],
    next_index: usize
}

impl Random for RandomSequence {

    fn new() -> RandomSequence {
        let nb = [0u8; 1024];
        RandomSequence {next_bytes: nb, next_index: 0}
    }

    fn fill_bytes(&mut self, out: &mut [u8]) {
        for count in 0..out.len() {
            out[count] = self.next_bytes[self.next_index];
            self.next_index += 1;
        }
    }
}

struct RandomZeros;

impl Random for RandomZeros {

    fn new() -> RandomZeros {
        RandomZeros
    }

    fn fill_bytes(&mut self, out: &mut [u8]) {
        for count in 0..out.len() {
            out[count] = 0;
        }
    }
}

pub fn copy_memory(data: &[u8], out: &mut [u8]) -> usize {
    for count in 0..data.len() {out[count] = data[count];}
    data.len()
}

#[test]
fn non_psk_test() {

    // Noise_XX round-trip randomized test
    {

        type  HS = HandshakeState<NoiseXX, Dh25519, CipherAESGCM, HashSHA256, RandomOs>;

        let mut rng_i = RandomOs::new();
        let mut rng_r = RandomOs::new();
        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut h_i = HS::new(rng_i, true, &[0u8;0], None, Some(static_i), None, None, None);
        let mut h_r = HS::new(rng_r, false, &[0u8;0], None, Some(static_r), None, None, None);

        let mut buffer = [0u8; 1024];

        // -> e
        let payload_0 = "abcdef".as_bytes();
        let (msg_0_len, _) = h_i.write_message(&payload_0, &mut buffer);
        assert!(msg_0_len == 38);

        let mut payload_0_out = [0u8; 1024];
        let result_0 = h_r.read_message(&buffer[..msg_0_len], &mut payload_0_out);
        let (payload_0_out_len, _) = result_0.unwrap(); 
        assert!(payload_0.len() == payload_0_out_len);
        assert!(payload_0.to_hex() == payload_0_out[..payload_0_out_len].to_hex());


        // <- e, dhee, s, dhse
        let payload_1 = [0u8; 0]; 
        let (msg_1_len, _) = h_r.write_message(&payload_1, &mut buffer);
        assert!(msg_1_len == 96);

        let mut payload_1_out = [0u8; 1024];
        let result_1 = h_i.read_message(&buffer[..msg_1_len], &mut payload_1_out);
        let (payload_1_out_len, _) = result_1.unwrap(); 
        assert!(payload_1.len() == payload_1_out_len);


        // -> s, dhse
        let payload_2 = "0123456789012345678901234567890123456789012345678901234567890123456789".as_bytes();
        let (msg_2_len, cipher_states_i_option) = h_i.write_message(&payload_2, &mut buffer);
        assert!(msg_2_len == 134);

        let mut payload_2_out = [0u8; 1024];
        let result_2 = h_r.read_message(&buffer[..msg_2_len], &mut payload_2_out);
        let (payload_2_out_len, cipher_states_r_option) = result_2.unwrap(); 
        assert!(payload_2.len() == payload_2_out_len);
        assert!(payload_2.to_hex() == payload_2_out[..payload_2_out_len].to_hex());

        let mut cipher_states_i = cipher_states_i_option.unwrap();
        let mut cipher_states_r = cipher_states_r_option.unwrap();

        // transport message I -> R
        let payload_3 = "wubba".as_bytes();
        cipher_states_i.0.encrypt(&payload_3, &mut buffer);

        let mut payload_3_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_3_out));
        assert!(payload_3.to_hex() == payload_3_out[..5].to_hex());

        // transport message I -> R again
        let payload_4 = "aleph".as_bytes();
        cipher_states_i.0.encrypt(&payload_4, &mut buffer);

        let mut payload_4_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_4_out));
        assert!(payload_4.to_hex() == payload_4_out[..5].to_hex());

        // transport message R <- I
        let payload_5 = "worri".as_bytes();
        cipher_states_i.0.encrypt(&payload_5, &mut buffer);

        let mut payload_5_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_5_out));
        assert!(payload_5.to_hex() == payload_5_out[..5].to_hex());
    } 

    // Noise_N test
    {
        type  HS = HandshakeState<NoiseN, Dh25519, CipherAESGCM, HashSHA256, RandomInc>;
        let mut rng = RandomInc::new();
        let static_r = Dh25519::generate(&mut rng);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h = HS::new(rng, true, &[0u8;0], None, None, None, Some(static_pubkey), None);
        let mut buffer = [0u8; 48];
        assert!(h.write_message(&[0u8;0], &mut buffer).0 == 48);
        //println!("{}", buffer.to_hex());
        assert!(buffer.to_hex() =="358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd1662548331a3d1e93b490263abc7a4633867f4"); 
    }

    // Noise_X test
    {
        type  HS = HandshakeState<NoiseX, Dh25519, CipherChaChaPoly, HashSHA256, RandomInc>;
        let mut rng = RandomInc::new();
        let static_i = Dh25519::generate(&mut rng);
        let static_r = Dh25519::generate(&mut rng);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h = HS::new(rng, true, &[0u8;0], None, Some(static_i), None, Some(static_pubkey), None);
        let mut buffer = [0u8; 96];
        assert!(h.write_message(&[0u8;0], &mut buffer).0 == 96);
        //println!("{}", buffer.to_hex());
        assert!(buffer.to_hex() == "79a631eede1bf9c98f12032cdeadd0e7a079398fc786b88cc846ec89af85a51ad203cd28d81cf65a2da637f557a05728b3ae4abdc3a42d1cda5f719d6cf41d7f2cf1b1c5af10e38a09a9bb7e3b1d589a99492cc50293eaa1f3f391b59bb6990d");
    } 

    // Noise_NN test
    {
        type  HS = HandshakeState<NoiseNN, Dh25519, CipherAESGCM, HashSHA512, RandomInc>;
        let rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 

        let mut h_i = HS::new(rng_i, true, &[0u8;0], None, None, None, None, None);
        let mut h_r = HS::new(rng_r, false, &[0u8;0], None, None, None, None, None);
        let mut buffer_msg = [0u8; 64];
        let mut buffer_out = [0u8; 10];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 35);
        assert!(h_r.read_message(&buffer_msg[..35], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 52);
        assert!(h_i.read_message(&buffer_msg[..52], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        //println!("{}", buffer_msg[..52].to_hex());
        assert!(buffer_msg[..52].to_hex() == "07a37cbc142093c8b755dc1b10e86cb426374ad16aa853ed0bdfc0b2b86d1c7c5e4dc9545d41b3280f4586a5481829e1e24ec5a0"); 
    } 

    // Noise_XX test
    {
        type  HS = HandshakeState<NoiseXX, Dh25519, CipherAESGCM, HashSHA256, RandomInc>;
        let mut rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h_i = HS::new(rng_i, true, &[0u8;0], None, Some(static_i), None, None, None);
        let mut h_r = HS::new(rng_r, false, &[0u8;0], None, Some(static_r), None, None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 35);
        assert!(h_r.read_message(&buffer_msg[..35], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 100);
        assert!(h_i.read_message(&buffer_msg[..100], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        assert!(h_i.write_message(&[0u8;0], &mut buffer_msg).0 == 64);
        assert!(h_r.read_message(&buffer_msg[..64], &mut buffer_out).unwrap().0 == 0);

        assert!(buffer_msg[..64].to_hex() == "8127f4b35cdbdf0935fcf1ec99016d1dcbc350055b8af360be196905dfb50a2c1c38a7ca9cb0cfe8f4576f36c47a4933eee32288f590ac4305d4b53187577be7");
    } 

    // Noise_IK test
    {
        type  HS = HandshakeState<NoiseIK, Dh25519, CipherAESGCM, HashSHA256, RandomInc>;
        let mut rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h_i = HS::new(rng_i, true, "ABC".as_bytes(), None, Some(static_i), None, Some(static_pubkey), None);
        let mut h_r = HS::new(rng_r, false, "ABC".as_bytes(), None, Some(static_r), None, None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 99);
        assert!(h_r.read_message(&buffer_msg[..99], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 52);
        assert!(h_i.read_message(&buffer_msg[..52], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        //println!("{}", buffer_msg[..52].to_hex());
        assert!(buffer_msg[..52].to_hex() == "5869aff450549732cbaaed5e5df9b30a6da31cb0e5742bad5ad4a1a768f1a67b7555a94199d0ce2972e0861b06c2152419a278de");
    } 

    // Noise_XE test
    {
        type  HS = HandshakeState<NoiseXE, Dh25519, CipherChaChaPoly, HashBLAKE2b, RandomInc>;
        let mut rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let eph_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);
        let mut eph_pubkey = [0u8; 32];
        copy_memory(eph_r.pubkey(), &mut eph_pubkey);

        let mut h_i = HS::new(rng_i, true, &[0u8;0], None, Some(static_i), None, Some(static_pubkey), Some(eph_pubkey));
        let mut h_r = HS::new(rng_r, false, &[0u8;0], None, Some(static_r), Some(eph_r), None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 51);
        assert!(h_r.read_message(&buffer_msg[..51], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 52);
        assert!(h_i.read_message(&buffer_msg[..52], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        assert!(h_i.write_message(&[0u8;0], &mut buffer_msg).0 == 64);
        assert!(h_r.read_message(&buffer_msg[..64], &mut buffer_out).unwrap().0 == 0);

        //println!("{}", buffer_msg[..64].to_hex());
        assert!(buffer_msg[..64].to_hex() == "08439f380b6f128a1465840d558f06abb1141cf5708a9dcf573d6e4fae01f90fd68dec89b26b249f2c4c61add5a1dbcf0a652ef015d7dbe0e80e9ea9af0aa7a2");
    } 


    // Noise_XX test with zeros
    {
        type  HS = HandshakeState<NoiseXX, Dh25519, CipherAESGCM, HashSHA256, RandomZeros>;
        let mut rng_i = RandomZeros::new();
        let mut rng_r = RandomZeros::new();

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h_i = HS::new(rng_i, true, "".as_bytes(), None, Some(static_i), None, None, None);
        let mut h_r = HS::new(rng_r, false, "".as_bytes(), None, Some(static_r), None, None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("".as_bytes(), &mut buffer_msg).0 == 32);
        assert!(h_r.read_message(&buffer_msg[..32], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("".as_bytes(), &mut buffer_msg).0 == 96);
        assert!(h_i.read_message(&buffer_msg[..96], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..4].to_hex() == "64656667");

        assert!(h_i.write_message("".as_bytes(), &mut buffer_msg).0 == 64);
        assert!(h_r.read_message(&buffer_msg[..64], &mut buffer_out).unwrap().0 == 0);


        //println!("{}", buffer_msg[..64].to_hex());
        assert!(buffer_msg[..64].to_hex() == "e98401c3c2cd0d167d492a41740000bc78ed5a47bcce3b32aacb08b2739b9969c98cf225d0d937656769e61c19e950b07b9fa73007b0a98a279c48040968a2af");
    } 

    // Noise_IK test with zeros
    {
        type  HS = HandshakeState<NoiseIK, Dh25519, CipherAESGCM, HashSHA256, RandomZeros>;
        let mut rng_i = RandomZeros::new();
        let mut rng_r = RandomZeros::new();

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);

        let mut h_i = HS::new(rng_i, true, "".as_bytes(), None, Some(static_i), None, Some(static_pubkey), None);
        let mut h_r = HS::new(rng_r, false, "".as_bytes(), None, Some(static_r), None, None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("".as_bytes(), &mut buffer_msg).0 == 96);
        assert!(h_r.read_message(&buffer_msg[..96], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..3].to_hex() == "");

        assert!(h_r.write_message("".as_bytes(), &mut buffer_msg).0 == 48);
        assert!(h_i.read_message(&buffer_msg[..48], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..4].to_hex() == "");

        //println!("{}", buffer_msg[..48].to_hex());
        assert!(buffer_msg[..48].to_hex() == "2fe57da347cd62431528daac5fbb290730fff684afc4cfc2ed90995f58cb3b745a1e05164a38bc5e0ed07a0c15871dae");
    } 

    // Noise_XXfallback test with zeros
    {
        type  HS = HandshakeState<NoiseXXfallback, Dh25519, CipherAESGCM, HashSHA256, RandomZeros>;
        let mut rng_i = RandomZeros::new();
        let mut rng_r = RandomZeros::new();

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let eph_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);
        let mut eph_pubkey = [0u8; 32];
        copy_memory(eph_r.pubkey(), &mut eph_pubkey);

        let mut h_i = HS::new(rng_i, true, "".as_bytes(), None, Some(static_i), None, None, Some(eph_pubkey));
        let mut h_r = HS::new(rng_r, false, "".as_bytes(), None, Some(static_r), Some(eph_r), None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("".as_bytes(), &mut buffer_msg).0 == 96);
        assert!(h_r.read_message(&buffer_msg[..96], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("".as_bytes(), &mut buffer_msg).0 == 64);
        assert!(h_i.read_message(&buffer_msg[..64], &mut buffer_out).unwrap().0 == 0);
        //assert!(buffer_out[..4].to_hex() == "64656667");

        //println!("{}", buffer_msg[..64].to_hex());
        assert!(buffer_msg[..64].to_hex() == "78c8860d6f147066ef956925de58379fbe6d49b9dd885b7ba3401f885b9bb31a9d9c7d136999fc5573369eb775cc027f8f1bd7f2b8a5f024c520ed91af85dbb4");
    } 

}

#[test]
fn psk_test() {

    // NoisePSK_XX round-trip randomized test
    {

        type  HS = HandshakeState<NoiseXX, Dh25519, CipherAESGCM, HashSHA256, RandomOs>;

        let mut rng_i = RandomOs::new();
        let mut rng_r = RandomOs::new();
        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let psk = [1u8, 2u8, 3u8];
        let mut h_i = HS::new(rng_i, true, &[0u8;0], Some(&psk), Some(static_i), None, None, None);
        let mut h_r = HS::new(rng_r, false, &[0u8;0], Some(&psk), Some(static_r), None, None, None);

        let mut buffer = [0u8; 1024];

        // -> e
        let payload_0 = "abcdef".as_bytes();
        let (msg_0_len, _) = h_i.write_message(&payload_0, &mut buffer);
        assert!(msg_0_len == 54);

        let mut payload_0_out = [0u8; 1024];
        let result_0 = h_r.read_message(&buffer[..msg_0_len], &mut payload_0_out);
        let (payload_0_out_len, _) = result_0.unwrap(); 
        assert!(payload_0.len() == payload_0_out_len);
        assert!(payload_0.to_hex() == payload_0_out[..payload_0_out_len].to_hex());


        // <- e, dhee, s, dhse
        let payload_1 = [0u8; 0]; 
        let (msg_1_len, _) = h_r.write_message(&payload_1, &mut buffer);
        assert!(msg_1_len == 96);

        let mut payload_1_out = [0u8; 1024];
        let result_1 = h_i.read_message(&buffer[..msg_1_len], &mut payload_1_out);
        let (payload_1_out_len, _) = result_1.unwrap(); 
        assert!(payload_1.len() == payload_1_out_len);


        // -> s, dhse
        let payload_2 = "0123456789012345678901234567890123456789012345678901234567890123456789".as_bytes();
        let (msg_2_len, cipher_states_i_option) = h_i.write_message(&payload_2, &mut buffer);
        assert!(msg_2_len == 134);

        let mut payload_2_out = [0u8; 1024];
        let result_2 = h_r.read_message(&buffer[..msg_2_len], &mut payload_2_out);
        let (payload_2_out_len, cipher_states_r_option) = result_2.unwrap(); 
        assert!(payload_2.len() == payload_2_out_len);
        assert!(payload_2.to_hex() == payload_2_out[..payload_2_out_len].to_hex());

        let mut cipher_states_i = cipher_states_i_option.unwrap();
        let mut cipher_states_r = cipher_states_r_option.unwrap();

        // transport message I -> R
        let payload_3 = "wubba".as_bytes();
        cipher_states_i.0.encrypt(&payload_3, &mut buffer);

        let mut payload_3_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_3_out));
        assert!(payload_3.to_hex() == payload_3_out[..5].to_hex());

        // transport message I -> R again
        let payload_4 = "aleph".as_bytes();
        cipher_states_i.0.encrypt(&payload_4, &mut buffer);

        let mut payload_4_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_4_out));
        assert!(payload_4.to_hex() == payload_4_out[..5].to_hex());

        // transport message R <- I
        let payload_5 = "worri".as_bytes();
        cipher_states_i.0.encrypt(&payload_5, &mut buffer);

        let mut payload_5_out = [0u8; 1024];
        assert!(cipher_states_r.1.decrypt(&buffer[..21], &mut payload_5_out));
        assert!(payload_5.to_hex() == payload_5_out[..5].to_hex());
    } 

    // NoisePSK_N test
    {
        type  HS = HandshakeState<NoiseN, Dh25519, CipherAESGCM, HashSHA256, RandomInc>;
        let mut rng = RandomInc::new();
        let static_r = Dh25519::generate(&mut rng);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);
        let psk = [1u8, 2u8, 3u8];

        let mut h = HS::new(rng, true, &[0u8;0], Some(&psk), None, None, Some(static_pubkey), None);
        let mut buffer = [0u8; 48];
        assert!(h.write_message(&[0u8;0], &mut buffer).0 == 48);
        //println!("{}", buffer.to_hex());
        assert!(buffer.to_hex() =="358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd16625475344a60649da3ec23ce8e3ed779e766"); 
    }

    // NoisePSK_X test
    {
        type  HS = HandshakeState<NoiseX, Dh25519, CipherChaChaPoly, HashSHA256, RandomInc>;
        let mut rng = RandomInc::new();
        let static_i = Dh25519::generate(&mut rng);
        let static_r = Dh25519::generate(&mut rng);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);
        let psk = [1u8, 2u8, 3u8];

        let mut h = HS::new(rng, true, &[0u8;0], Some(&psk), Some(static_i), None, Some(static_pubkey), None);
        let mut buffer = [0u8; 96];
        assert!(h.write_message(&[0u8;0], &mut buffer).0 == 96);
        //println!("{}", buffer.to_hex());
        assert!(buffer.to_hex() == "79a631eede1bf9c98f12032cdeadd0e7a079398fc786b88cc846ec89af85a51a12d5cf01bc576e8f0124b14db3ed7a00d20f16186e8f1e2c861fb3d4113f39b290f0048404b8d21e2098958b6bdf50f41dfb1143700310482cfb52c9002261bd");
    } 

    // NoisePSK_NN test (prologue AND psk)
    {
        type  HS = HandshakeState<NoiseNN, Dh25519, CipherAESGCM, HashSHA512, RandomInc>;
        let rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 
        let prologue = [1u8, 2u8, 3u8];
        let psk = [4u8, 5u8, 6u8];


        let mut h_i = HS::new(rng_i, true, &prologue, Some(&psk), None, None, None, None);
        let mut h_r = HS::new(rng_r, false, &prologue, Some(&psk), None, None, None, None);
        let mut buffer_msg = [0u8; 64];
        let mut buffer_out = [0u8; 10];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 51);
        assert!(h_r.read_message(&buffer_msg[..51], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 52);
        assert!(h_i.read_message(&buffer_msg[..52], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        //println!("{}", buffer_msg[..52].to_hex());
        assert!(buffer_msg[..52].to_hex() == "07a37cbc142093c8b755dc1b10e86cb426374ad16aa853ed0bdfc0b2b86d1c7cfda657b21e8eac78df67b6bd453c0b11372364a6"); 
    } 

    // NoisePSK_XX test
    {
        type  HS = HandshakeState<NoiseXX, Dh25519, CipherAESGCM, HashSHA256, RandomInc>;
        let mut rng_i = RandomInc::new();
        let mut rng_r = RandomInc::new();
        rng_r.next_byte = 1; 

        let static_i = Dh25519::generate(&mut rng_i);
        let static_r = Dh25519::generate(&mut rng_r);
        let mut static_pubkey = [0u8; 32];
        copy_memory(static_r.pubkey(), &mut static_pubkey);
        let prologue = [1u8, 2u8, 3u8];
        let psk = [4u8, 5u8, 6u8];

        let mut h_i = HS::new(rng_i, true, &prologue, Some(&psk), Some(static_i), None, None, None);
        let mut h_r = HS::new(rng_r, false, &prologue, Some(&psk), Some(static_r), None, None, None);
        let mut buffer_msg = [0u8; 200];
        let mut buffer_out = [0u8; 200];
        assert!(h_i.write_message("abc".as_bytes(), &mut buffer_msg).0 == 51);
        assert!(h_r.read_message(&buffer_msg[..51], &mut buffer_out).unwrap().0 == 3);
        assert!(buffer_out[..3].to_hex() == "616263");

        assert!(h_r.write_message("defg".as_bytes(), &mut buffer_msg).0 == 100);
        assert!(h_i.read_message(&buffer_msg[..100], &mut buffer_out).unwrap().0 == 4);
        assert!(buffer_out[..4].to_hex() == "64656667");

        assert!(h_i.write_message(&[0u8;0], &mut buffer_msg).0 == 64);
        assert!(h_r.read_message(&buffer_msg[..64], &mut buffer_out).unwrap().0 == 0);

        //println!("{}", buffer_msg[..64].to_hex());
        assert!(buffer_msg[..64].to_hex() == "2b9c628158a517e3984dc619245d4b9cd73561944f266181b183812ca73499881e30f6e7eeb576c258acc713c2c62874fd1beb76b122f6303f974109aefd7e2a");
    } 
}

