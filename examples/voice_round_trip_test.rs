use opus_cpal_bridge::{
    opus_codec::{
        decoder::{DecodeInput, Decoder},
        encoder::Encoder,
    },
    resampler::Resampler,
    stream::{Input, Output, Stream},
};
use ringbuf::{
    HeapCons, HeapProd, HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};
use tracing::info;

const RINGBUF_SIZE: usize = 4 * 1024;

#[test]
fn voice_round_trip() {
    tracing_subscriber::fmt().init();

    let host = cpal::default_host();
    let (packet_producer, packet_consumer) = HeapRb::<(usize, [u8; 4000])>::new(16).split();
    let (input, consumer) = Stream::<Input>::new(&host, RINGBUF_SIZE).unwrap();
    let (output, producer) = Stream::<Output>::new(&host, RINGBUF_SIZE).unwrap();
    let _handle = std::thread::spawn(move || output_loop(output, producer, packet_consumer));
    input_loop(input, consumer, packet_producer);
}

fn input_loop(
    input: Stream<Input>,
    mut consumer: HeapCons<f32>,
    mut packet_producer: HeapProd<(usize, [u8; 4000])>,
) {
    let mut resampler = Resampler::new(&input).expect("Failed to construct input resampler");
    let mut encoder = Encoder::new().expect("Failed to construct encoder");
    let mut buf_in = vec![0.0f32; usize::from(u16::MAX)];
    let mut n_in: usize = 0;
    let mut buf_out = vec![0.0f32; usize::from(u16::MAX)];
    let mut n_out: usize = 0;
    info!("Input Resampler max input size {}", resampler.max_input());
    info!("Input Resampler max output size {}", resampler.max_output());
    let mut packet = vec![0u8; 4000];

    loop {
        std::thread::park_timeout(std::time::Duration::from_millis(1));

        let input_frames_next = resampler.input_frames_next();
        if consumer.occupied_len() < input_frames_next {
            continue;
        }
        let popped = consumer.pop_slice(&mut buf_in[n_in..n_in + input_frames_next]);
        n_in = n_in.strict_add(popped);
        assert!(popped == input_frames_next);

        let input_frames_next = resampler.input_frames_next();
        if n_in < input_frames_next {
            continue;
        }
        let (consumed, produced) = resampler
            .resample(&buf_in[..input_frames_next], &mut buf_out[n_out..])
            .expect("Failed to resample input");
        assert!(consumed == popped);
        n_in = n_in.strict_sub(consumed);
        n_out = n_out.strict_add(produced);

        let input_sample_len = encoder.input_sample_len();
        if n_out < input_sample_len {
            continue;
        }
        let len = encoder
            .encode(&buf_out[..input_sample_len], &mut packet)
            .expect("Failed to encode samples");
        n_out = n_out.strict_sub(input_sample_len);

        let mut out_packet = [0u8; 4000];
        out_packet[..len].copy_from_slice(&packet[..len]);
        let _ = packet_producer
            .try_push((len, out_packet))
            .expect("Failed to push packet to ring buf");
        info!("Pushed {len} onto the packet wire");
    }
}

fn output_loop(
    output: Stream<Output>,
    mut producer: HeapProd<f32>,
    mut packet_consumer: HeapCons<(usize, [u8; 4000])>,
) {
    let mut resampler = Resampler::new(&output).expect("Failed to construct output resampler");
    let mut decoder = Decoder::new().expect("Failed to construct decoder");
    let mut buf_in = vec![0.0f32; usize::from(u16::MAX)];
    let mut n_in: usize = 0;
    let mut buf_out = vec![0.0f32; usize::from(u16::MAX)];
    let mut n_out: usize = 0;

    loop {
        std::thread::park_timeout(std::time::Duration::from_millis(1));

        let (len, packet) = match packet_consumer.try_pop() {
            Some(v) => v,
            None => continue,
        };
        info!("Received {len} from the wire");

        let len = decoder
            .decode(DecodeInput::Packet(&packet[..len]), &mut buf_in[n_in..])
            .expect("Failed to decode packet");
        n_in = n_in.strict_add(len);

        let input_frames_next = resampler.input_frames_next();
        if n_in < input_frames_next {
            continue;
        }
        let (consumed, produced) = resampler
            .resample(&buf_in[..input_frames_next], &mut buf_out[n_out..])
            .expect("Failed to resample samples");
        n_in = n_in.strict_sub(consumed);
        n_out = n_out.strict_add(produced);

        let pushed = producer.push_slice(&buf_out[..n_out]);
        buf_out.copy_within(pushed..n_out, 0);
        n_out = n_out.strict_sub(pushed);
    }
}
