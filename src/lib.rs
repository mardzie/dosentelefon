use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex, RwLock},
    thread::{self, JoinHandle},
};

use crossbeam::channel::Receiver;
use ringbuf::traits::{Consumer, Observer};

use crate::{
    audio::{Audio, InputMessage, audio_processor::AudioProcessor},
    types::{ArcMutex, ArcRwLock},
    udp_packet_net::MAX_PAYLOAD_SIZE,
    voice_net::VoiceNet,
};

mod audio;
mod hash;
mod helpers;
mod types;
mod udp_packet_net;
mod voice_net;

pub const AUDIO_CHANNELS_BUF_SIZE: usize = 1024 * 16;
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(10);

#[derive(Debug)]
pub struct VChat {
    audio: Arc<Audio>,
    voice_net: ArcMutex<VoiceNet>,
    addresses: ArcRwLock<Vec<SocketAddr>>,

    exit_notify: crossbeam::channel::Sender<InputMessage>,
    input_udp_bridge_handle: JoinHandle<Result<(), ()>>,
    output_udp_bridge_handle: JoinHandle<Result<(), ()>>,
}

impl VChat {
    pub fn new<A>(addr: A) -> Self
    where
        A: ToSocketAddrs,
    {
        let (exit_notify, input_notify_rx) = crossbeam::channel::bounded(128);
        let (speaker_input_tx, speaker_output_rx) =
            crossbeam::channel::bounded(AUDIO_CHANNELS_BUF_SIZE);

        let input_notify_tx_c = exit_notify.clone();
        let (audio, mut consumer) = Audio::new(input_notify_tx_c, speaker_output_rx, u8::MAX);
        let audio = Arc::new(audio);

        let voice_net = Arc::new(Mutex::new(
            VoiceNet::new(addr).expect("Failed to create VoiceNet."),
        ));

        let addresses = Arc::new(RwLock::new(Vec::with_capacity(8)));

        let audio_processor = audio.audio_processor();
        let voice_net_c = voice_net.clone();
        let addresses_c = addresses.clone();
        let input_udp_bridge_handle = thread::Builder::new()
            .name("Input UDP Bridge".to_string())
            .spawn(move || {
                tracing::debug!("Input UDP Bridge started...");
                loop {
                    if Self::input_udp_bridge(
                        &voice_net_c,
                        &audio_processor,
                        &input_notify_rx,
                        &mut consumer,
                        &addresses_c,
                    )
                    .is_err()
                    {
                        break;
                    };
                }
                tracing::debug!("Input UDP Bridge stopped.");

                Err(())
            })
            .expect("Failed to build UDP Bridge thread!");

        let voice_net_c = voice_net.clone();
        let output_udp_bridge_handle = thread::Builder::new()
            .name("Output UDP Bridge".to_string())
            .spawn(move || {
                tracing::debug!("Output UDP Bridge started...");
                loop {
                    if Self::udp_output_bridge(&voice_net_c, &speaker_input_tx).is_err() {
                        break;
                    };
                }
                tracing::debug!("Output UDP Bridge stopped.");

                Err(())
            })
            .expect("Failed to build UDP Bridge thread!");

        audio.play();

        Self {
            audio,
            voice_net,
            addresses,

            exit_notify,
            input_udp_bridge_handle,
            output_udp_bridge_handle,
        }
    }

    fn input_udp_bridge(
        voice_net: &ArcMutex<VoiceNet>,
        audio_processor: &Arc<AudioProcessor>,
        input_notify: &Receiver<InputMessage>,
        input_ringbuf: &mut ringbuf::wrap::caching::Caching<
            Arc<ringbuf::SharedRb<ringbuf::storage::Heap<f32>>>,
            false,
            true,
        >,
        addresses: &ArcRwLock<Vec<SocketAddr>>,
    ) -> Result<(), ()> {
        match input_notify.recv() {
            Ok(InputMessage::Samples) => {}
            Ok(InputMessage::Exit) => return Err(()),
            Err(_) => return Err(()),
        };

        let mut data = vec![0f32; input_ringbuf.occupied_len()];
        input_ringbuf.pop_slice(&mut data);
        input_ringbuf.pop_iter();

        let data = audio_processor.process_audio(data);

        // Convert into bytes and split up into packets.
        let byte_packets: Vec<Vec<u8>> = data
            .chunks(MAX_PAYLOAD_SIZE / 4)
            .map(|chunk| {
                chunk
                    .iter()
                    .flat_map(|sample| sample.to_be_bytes())
                    .collect::<Vec<u8>>()
            })
            .collect();

        if !byte_packets.is_empty() {
            tracing::trace!(
                "Input UDP Bridge: Preparing {} packet {} bytes.",
                byte_packets.len(),
                byte_packets[0].len()
            );
        }

        let voice_net_lock = voice_net
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let addresses_lock = addresses
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for bytes in byte_packets {
            for addr in addresses_lock.iter() {
                if let Err(e) = voice_net_lock.send(bytes.clone(), addr) {
                    match e {
                        voice_net::error::SendError::WouldBlock => {
                            tracing::warn!("Input UDP Bridge: Failed to send `Packet` to {}", addr);
                        }
                        voice_net::error::SendError::Io(e) => {
                            tracing::warn!(
                                "Inptu UDP Bridge: Failed to send `Packet` to {}: {}",
                                addr,
                                e
                            );
                        }
                    };
                };
            }
        }

        Ok(())
    }

    fn udp_output_bridge(
        voice_net: &ArcMutex<VoiceNet>,
        output_tx: &crossbeam::channel::Sender<Vec<f32>>,
    ) -> Result<(), ()> {
        const AUDIO_VALUE_BYTE_LEN: usize = 4;

        let packet: crate::voice_net::packets::Packet = {
            let mut voice_net_lock = voice_net
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(buf_packet) = voice_net_lock.recv() {
                buf_packet.into()
            } else {
                return Ok(());
            }
        };
        let (src_addr, payload) = packet.inner();

        tracing::trace!(
            "UDP Output Bridge: Got message from {} with size {}",
            src_addr,
            payload.len()
        );

        let samples: Vec<f32> = payload
            .chunks(AUDIO_VALUE_BYTE_LEN)
            .map(|chunk| {
                let mut buf = [0u8; AUDIO_VALUE_BYTE_LEN];
                buf.copy_from_slice(chunk);

                buf
            })
            .map(f32::from_be_bytes)
            .collect();

        match output_tx.send(samples) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("UDP Output Bridge: Output device closed channel: {}", e);
                return Err(());
            }
        };

        Ok(())
    }

    pub fn add_address(&self, addr: SocketAddr) {
        let mut addresses = self
            .addresses
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        addresses.push(addr);
        addresses.sort();
        addresses.dedup();
    }

    #[inline]
    pub fn get_addresses(&self) -> &std::sync::Arc<std::sync::RwLock<Vec<SocketAddr>>> {
        &self.addresses
    }

    pub fn remove_address(&self, addr: &SocketAddr) -> Option<SocketAddr> {
        let mut lock = self
            .addresses
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let idx = lock
            .iter()
            .position(|internal_addr| addr == internal_addr)?;
        Some(lock.remove(idx))
    }

    #[inline]
    pub fn clear_addresses(&self) {
        self.addresses
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    #[inline]
    pub fn audio(&self) -> &Audio {
        &self.audio
    }

    #[inline]
    pub fn voice_net(&self) -> &ArcMutex<VoiceNet> {
        &self.voice_net
    }

    pub fn stop(self) {
        if let Err(e) = self.exit_notify.send(InputMessage::Exit) {
            tracing::error!("Failed to send exit notification to audio thread: {}", e);
        };

        let _ = self.input_udp_bridge_handle.join();
        let _ = self.output_udp_bridge_handle.join();
    }
}
