use std::hint::black_box;

use bytes::BytesMut;
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use ferrite_protocol::cipher::{Decryptor, Encryptor};
use ferrite_protocol::varint;
use ferrite_protocol::{Compression, FrameDecoder, FrameEncoder};
use ferrite_protocol::{Encode, VarInt};

fn noise(len: usize) -> Vec<u8> {
    let mut x = 0x9e3779b9u32;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

/// Chunk-like data: long runs and repeats, like palettised block data.
fn compressible(len: usize) -> Vec<u8> {
    (0..len).map(|i| ((i / 64) % 7) as u8).collect()
}

fn bench_varint(c: &mut Criterion) {
    let values: Vec<i32> = (0..1024).map(|i| (i * 2_654_435) ^ (i << (i % 24))).collect();
    let mut g = c.benchmark_group("varint");
    g.throughput(Throughput::Elements(values.len() as u64));
    g.bench_function("encode", |b| {
        let mut out = Vec::with_capacity(8192);
        b.iter(|| {
            out.clear();
            for &v in &values {
                VarInt(black_box(v)).encode(&mut out).unwrap();
            }
            black_box(&out);
        })
    });
    g.bench_function("encode/array+copy", |b| {
        let mut out = Vec::with_capacity(8192);
        b.iter(|| {
            out.clear();
            for &v in &values {
                let (buf, n) = varint::encode_i32(black_box(v));
                out.extend_from_slice(&buf[..n]);
            }
            black_box(&out);
        })
    });
    let mut encoded = Vec::new();
    for &v in &values {
        let (buf, n) = varint::encode_i32(v);
        encoded.extend_from_slice(&buf[..n]);
    }
    g.bench_function("decode", |b| {
        b.iter(|| {
            let mut s = &encoded[..];
            let mut sum = 0i32;
            while !s.is_empty() {
                sum = sum.wrapping_add(varint::read_i32(&mut s).unwrap());
            }
            black_box(sum)
        })
    });
    g.finish();
}

fn bench_cipher(c: &mut Criterion) {
    let key = [3u8; 16];
    for (name, size) in [("1KiB", 1024usize), ("64KiB", 64 * 1024)] {
        let mut g = c.benchmark_group(format!("cfb8/{name}"));
        g.throughput(Throughput::Bytes(size as u64));
        let data = noise(size);
        g.bench_function("encrypt", |b| {
            let mut enc = Encryptor::new(&key);
            let mut buf = data.clone();
            b.iter(|| enc.apply(black_box(&mut buf)))
        });
        g.bench_function("decrypt", |b| {
            let mut dec = Decryptor::new(&key);
            let mut buf = data.clone();
            b.iter(|| dec.apply(black_box(&mut buf)))
        });
        g.finish();
    }
}

fn bench_frames(c: &mut Criterion) {
    for (label, body) in [("small-32B", noise(32)), ("chunk-like-16KiB", compressible(16 * 1024))] {
        let mut g = c.benchmark_group(format!("frame/{label}"));
        g.throughput(Throughput::Bytes(body.len() as u64));

        for (mode, compression) in
            [("plain", None), ("zlib-thr-256", Some(Compression { threshold: 256 }))]
        {
            let mut enc = FrameEncoder::new();
            enc.set_compression(compression, 4).unwrap();
            g.bench_function(format!("encode/{mode}"), |b| {
                let mut out = BytesMut::with_capacity(64 * 1024);
                b.iter(|| {
                    out.clear();
                    enc.encode(0x2c, black_box(&body), &mut out).unwrap();
                    black_box(&out);
                })
            });

            let mut wire = BytesMut::new();
            enc.encode(0x2c, &body, &mut wire).unwrap();
            g.bench_function(format!("decode/{mode}"), |b| {
                let mut dec = FrameDecoder::new();
                dec.set_compression(compression);
                b.iter_batched(
                    || (),
                    |_| {
                        dec.queue(&wire);
                        black_box(dec.next_frame().unwrap().unwrap())
                    },
                    BatchSize::SmallInput,
                )
            });
        }
        g.finish();
    }
}

fn bench_construction(c: &mut Criterion) {
    use ferrite_protocol::session::{ServerSession, SessionConfig};
    let mut g = c.benchmark_group("construction");
    g.bench_function("FrameEncoder::new", |b| b.iter(|| black_box(FrameEncoder::new())));
    g.bench_function("FrameDecoder::new", |b| b.iter(|| black_box(FrameDecoder::new())));
    g.bench_function("ServerSession::new", |b| {
        let cfg = SessionConfig::default();
        b.iter(|| black_box(ServerSession::new(cfg.clone())))
    });
    g.finish();
}

criterion_group!(benches, bench_varint, bench_cipher, bench_frames, bench_construction);
criterion_main!(benches);
