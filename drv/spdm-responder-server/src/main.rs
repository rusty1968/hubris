// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! SPDM Responder Server - Minimal version for IPC testing

#![no_std]
#![no_main]

use drv_spdm_responder_api::*;
use idol_runtime::{NotificationHandler, RequestError, Leased, W};
use ringbuf::{ringbuf, ringbuf_entry};
use userlib::*;

// Include generated server support
include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));

#[derive(Copy, Clone, PartialEq)]
enum Trace {
    None,
    GetVersion,
    GetCapabilities,
    NegotiateAlgorithms,
    GetCertificate { slot: u8, offset: u16, length: u16 },
    ChallengeAuth { slot: u8 },
    GetMeasurements { operation: u8, slot: u8 },
    KeyExchange { slot: u8 },
    PskExchange,
    Heartbeat,
    EndSession,
}

ringbuf!(Trace, 64, Trace::None);

/// Minimal SPDM responder implementation for IPC testing
struct TestSpdmResponder;

impl TestSpdmResponder {
    fn new() -> Self {
        Self
    }
}

impl idol_runtime::NotificationHandler for TestSpdmResponder {
    fn current_notification_mask(&self) -> u32 {
        0
    }

    fn handle_notification(&mut self, _bits: u32) {
        // No notifications to handle for testing
    }
}

impl InOrderSpdmResponderImpl for TestSpdmResponder {
    fn get_version(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<SpdmVersionResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::GetVersion);
        Ok(SpdmVersionResponse::default())
    }

    fn get_capabilities(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<SpdmCapabilities, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::GetCapabilities);
        Ok(SpdmCapabilities {
            ct_exponent: 12,
            flags: 0x01,
        })
    }

    fn negotiate_algorithms(
        &mut self,
        _msg: &RecvMessage,
        algorithms: AlgorithmRequest,
    ) -> Result<AlgorithmResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::NegotiateAlgorithms);
        Ok(AlgorithmResponse {
            base_asym_sel: algorithms.base_asym_algo & 0x1, // Select first supported
            base_hash_sel: algorithms.base_hash_algo & 0x1, // Select first supported
        })
    }

    fn get_certificate(
        &mut self,
        _msg: &RecvMessage,
        slot: u8,
        offset: u16,
        length: u16,
        buffer: Leased<W, [u8]>,
    ) -> Result<u16, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::GetCertificate { slot, offset, length });

        // Mock certificate data for testing
        let cert_data = b"MOCK_CERTIFICATE_FOR_TESTING";
        let cert_len = cert_data.len() as u16;

        if offset >= cert_len {
            return Ok(0);
        }

        let start = offset as usize;
        let available = cert_len - offset;
        let copy_len = core::cmp::min(length, available) as usize;
        let end = start + copy_len;

        buffer.write_range(0..copy_len, &cert_data[start..end])
            .map_err(|_| RequestError::went_away())?;

        Ok(copy_len as u16)
    }

    fn challenge_auth(
        &mut self,
        _msg: &RecvMessage,
        slot: u8,
        measurement_summary: u8,
        nonce: &[u8],
        signature: Leased<W, [u8]>,
    ) -> Result<ChallengeAuthResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::ChallengeAuth { slot });

        let mut response_nonce = [0u8; 32];
        if nonce.len() >= 32 {
            response_nonce.copy_from_slice(&nonce[..32]);
        }

        let mock_signature = [0x42u8; 32]; // Mock signature
        signature.write_range(0..32, &mock_signature)
            .map_err(|_| RequestError::went_away())?;

        Ok(ChallengeAuthResponse {
            cert_chain_hash: [0x12u8; 32],
            nonce: response_nonce,
            signature: mock_signature,
        })
    }

    fn get_measurements(
        &mut self,
        _msg: &RecvMessage,
        measurement_operation: u8,
        slot: u8,
        content_changed: u8,
    ) -> Result<MeasurementResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::GetMeasurements { operation: measurement_operation, slot });

        Ok(MeasurementResponse {
            measurement_index: slot,
            measurement_hash: [0x34u8; 32],
        })
    }

    fn key_exchange(
        &mut self,
        _msg: &RecvMessage,
        slot: u8,
        session_id: u16,
        random_data: &[u8],
        exchange_data: &[u8],
    ) -> Result<KeyExchangeResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::KeyExchange { slot });

        Ok(KeyExchangeResponse {
            heartbeat_period: 60,
            public_key: [0x56u8; 32],
        })
    }

    fn psk_exchange(
        &mut self,
        _msg: &RecvMessage,
        measurement_summary: u8,
        session_id: u16,
        psk_hint: &[u8],
        context: &[u8],
        exchange_data: &[u8],
    ) -> Result<PskExchangeResponse, RequestError<SpdmError>> {
        ringbuf_entry!(Trace::PskExchange);

        Ok(PskExchangeResponse {
            heartbeat_period: 30,
            context: [0x78u8; 32],
        })
    }

    fn heartbeat(
        &mut self,
        _msg: &RecvMessage,
        session_id: u16,
    ) -> Result<(), RequestError<SpdmError>> {
        ringbuf_entry!(Trace::Heartbeat);
        Ok(())
    }

    fn end_session(
        &mut self,
        _msg: &RecvMessage,
        session_id: u16,
    ) -> Result<(), RequestError<SpdmError>> {
        ringbuf_entry!(Trace::EndSession);
        Ok(())
    }
}

#[no_mangle]
fn main() -> ! {
    let mut server = TestSpdmResponder::new();

    let mut incoming = [0u8; 1024]; // Fixed buffer size for testing

    loop {
        idol_runtime::dispatch(&mut incoming, &mut server);
    }
}