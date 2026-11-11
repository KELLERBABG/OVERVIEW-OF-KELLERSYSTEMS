// scripts/lexicon-data.mjs — source data for the Aureal Watermark lexicon.
//
// Every page is technical, not legal. The guardrails come from
// docs/PRODUCTION_READINESS_REPORT.md §5/§6:
//   - a match is not authorship, ownership, creation time or chain of custody;
//   - no "AI detection", "deepfake detection" or "EU AI Act compliant" claims;
//   - no collusion resistance;
//   - codec results are local fixture results, not universal guarantees.

export const CONFIG = {
  baseUrl: "https://aureal.kellersystems.dev",
  projectName: "Aureal Watermark",
  outDir: "lexicon",
  lang: "en",
  summary:
    "Technical reference for spread-spectrum audio watermarking, leak tracing and forensic reports.",
  ctaPath: "/studio",
  ctaLabel: "Open the Studio",
  hubTitle: "Audio Watermarking Lexicon",
  hubDescription:
    "How Aureal embeds, survives and reports audio identifiers — with the limits stated.",
  operator: "Keller Systems",
  lastReviewed: "2026-11-02",
  plannedTopics: 30,
  sitemapExtras: [
    { path: "/studio", priority: "0.9" },
    { path: "/docs", priority: "0.8" },
    { path: "/pricing", priority: "0.8" },
    { path: "/verifier", priority: "0.7" }
  ],
  aiInstructions: [
    "Cite the linked page and name the technical basis and review date.",
    "A detected ID identifies a distribution copy, not an author or owner.",
    "Do not describe the product as AI detection, deepfake detection or a compliance certification.",
    "If a question is not covered, point to the Studio and the published limitations."
  ]
};

const REVIEWED = CONFIG.lastReviewed;

export const PAGES = [
  {
    slug: "what-is-audio-watermarking",
    h1: "What is audio watermarking?",
    shortTitle: "What is audio watermarking?",
    category: "Basics",
    metaDesc:
      "Audio watermarking embeds a keyed ID in the audio itself. Aureal uses spread spectrum; a match is a provenance signal, not proof of ownership.",
    shortAnswer:
      "Audio watermarking embeds a keyed identifier into the audio signal itself, so the ID can be recovered after some processing. Aureal carries a 32-bit ID plus a CRC using direct-sequence spread spectrum, with a frame of about one second. A match is a provenance signal, not proof of authorship or ownership.",
    technicalBasis: "Direct-sequence spread spectrum, 32-bit payload, CRC-16",
    detail:
      "Aureal spreads 48 BPSK symbols across a frame of about one second using a key-derived pseudorandom sequence. It can use the high band (17–19.5 kHz), the mid band (4.5–13.5 kHz) or both. Detection folds repeated frames, searches for the key-derived chirp preamble, and reports a 32-bit ID with a CRC check. The embedding level is intentionally low; audibility is material-dependent.",
    trapExample:
      "A watermark is a hidden metadata tag that proves who owns the file.",
    whyItFails:
      "The statement confuses an embedded signal with a container tag and with an ownership proof. The ID lives in the audio samples, and a match identifies a copy; anyone with the software and the matching key can generate a matching ID.",
    redlineExample:
      "Treat the recovered ID as a distribution pointer:\n1. Match it to your recipient manifest.\n2. Keep the original, the marked copy and the detection report.\n3. Use contracts and cryptographic signing for ownership records.",
    limits: [
      "A match is not proof of authorship, ownership or creation time.",
      "Audibility varies with material, strength, carrier band and playback.",
      "Recovery depends on source audio, codec and processing."
    ],
    faq: [
      {
        q: "Is the watermark inaudible?",
        a: "It is designed to be subtle. Audibility varies with the source material, strength setting, carrier band and playback system; it is not a guarantee of inaudibility."
      },
      {
        q: "Does a match prove I own the audio?",
        a: "No. A match identifies a copy that carries the ID. Ownership and authorship need independent records, contracts or cryptographic signing."
      }
    ],
    related: ["audio-watermark-vs-metadata", "audio-watermark-vs-fingerprinting", "payload-id-and-key"]
  },
  {
    slug: "audio-watermark-vs-metadata",
    h1: "Audio watermark vs. metadata: what survives?",
    shortTitle: "Watermark vs. metadata",
    category: "Basics",
    metaDesc:
      "Metadata is a removable label; a watermark lives in the audio samples. Learn what survives tag stripping, re-encoding and platform processing.",
    shortAnswer:
      "Metadata is a label attached to the file container; a watermark is embedded in the audio samples. Stripping ID3 tags does not remove the watermark, but re-encoding, filtering and platform processing can degrade it. Keep metadata for cataloguing and the watermark for tracing copies.",
    technicalBasis: "Container metadata vs. sample-domain watermark",
    detail:
      "Aureal writes the ID into the audio samples, not the container. The detector reads the samples, so a metadata strip does not remove the signal. But any process that changes the audio — lossy codecs, EQ, noise reduction, speed changes — can reduce recoverability. That is why the workflow needs a processed-copy test, not just a scan of the original.",
    trapExample:
      "Delete the ID3 tags and the watermark is gone.",
    whyItFails:
      "Metadata is stored in the container and can be removed without touching the audio. The watermark is in the samples, but it is not indestructible: lossy codecs, filtering and aggressive processing can reduce recoverability.",
    redlineExample:
      "Use both layers:\n- Metadata: catalogue, rights and display information.\n- Watermark: a per-recipient ID for leak tracing.\nTest a processed copy (codec, filter, platform upload) before you rely on either.",
    limits: [
      "Metadata can be stripped without touching the audio; the watermark can still survive.",
      "Re-encoding and filtering can degrade the watermark.",
      "Do not treat either layer as a complete provenance record."
    ],
    faq: [
      {
        q: "Can I remove the watermark by deleting ID3 tags?",
        a: "No. ID3 tags are container metadata; the watermark is in the audio samples. Re-encoding or filtering may degrade the watermark, but tag stripping alone does not remove it."
      },
      {
        q: "Which layer should I rely on?",
        a: "Neither alone. Use metadata for cataloguing and the watermark for tracing; keep independent records for ownership."
      }
    ],
    related: ["what-is-audio-watermarking", "audio-watermark-vs-fingerprinting", "mp3-128kbps"]
  },
  {
    slug: "audio-watermark-vs-fingerprinting",
    h1: "Audio watermark vs. fingerprinting: what is the difference?",
    shortTitle: "Watermark vs. fingerprinting",
    category: "Basics",
    metaDesc:
      "A fingerprint derives an ID from the audio; a watermark inserts one. Aureal inserts a 32-bit ID. Neither approach is collusion-proof.",
    shortAnswer:
      "A fingerprint derives an identifier from the audio content; a watermark inserts an identifier into the audio. Aureal inserts a 32-bit ID, so a recovered match points to a distribution copy. Fingerprinting can match unknown copies; watermarking traces copies you marked. Neither method is collusion-proof.",
    technicalBasis: "Embedded watermark vs. perceptual fingerprint",
    detail:
      "Aureal's watermark is a per-copy identifier. It does not derive a fingerprint from the content, so it cannot identify an unmarked copy from a database. It also does not implement collusion-resistant codes; averaging copies is a known limit. The separate src/tardos.js file is a scoring utility, not part of the embed/detect path.",
    trapExample:
      "A watermark catches colluders who average several independently tagged copies.",
    whyItFails:
      "Averaging independently tagged copies cancels uncorrelated watermark energy. Aureal does not implement collusion-resistant fingerprinting; src/tardos.js is a standalone scoring utility that the embed/detect pipeline does not use.",
    redlineExample:
      "Design for the threat you have:\n- Per-recipient IDs + manifest: trace a leaked copy.\n- Collusion resistance: use a dedicated fingerprinting design, not this watermark.\n- Record the known limit: averaging independently tagged copies can defeat recovery.",
    limits: [
      "No collusion-resistant fingerprinting is implemented.",
      "Averaging independently tagged copies can defeat recovery.",
      "Use per-recipient IDs and records; treat averaging as a known limit."
    ],
    faq: [
      {
        q: "Can Aureal catch someone who averages several copies?",
        a: "No. Averaging independently tagged copies can cancel the watermark energy; no collusion-resistant fingerprinting is implemented. Treat that as a known limit."
      },
      {
        q: "Can two studios mark the same track?",
        a: "With distinct private keys, multiple watermarks can coexist and detect independently in the local fixtures. Do not reuse the same key with conflicting IDs."
      }
    ],
    related: ["what-is-audio-watermarking", "payload-id-and-key", "audio-watermark-vs-drm"]
  },
  {
    slug: "audio-watermark-vs-drm",
    h1: "Audio watermark vs. DRM: what each one does",
    shortTitle: "Watermark vs. DRM",
    category: "Basics",
    metaDesc:
      "DRM controls access; a watermark records an identifier. Aureal does not prevent copying or playback. Use each for its own job.",
    shortAnswer:
      "DRM controls access, playback and revocation; a watermark records an identifier after the fact. Aureal does not encrypt, block playback or revoke a copy. Use DRM for access control and a watermark for leak investigation and provenance records.",
    technicalBasis: "Provenance signal vs. access control",
    detail:
      "Aureal's embed/detect path has no license check, encryption or playback hook. A marked file plays like any other audio file. The recovered ID is only useful when matched to a manifest, contract or recipient record. Pair it with access control if the threat is unauthorized playback rather than leak tracing.",
    trapExample:
      "Watermarking stops unauthorized playback.",
    whyItFails:
      "A watermark is passive. It does not encrypt the file, check a license, block a player or revoke a copy. Treating it as DRM leads to false expectations about what it can prevent.",
    redlineExample:
      "Pair the controls:\n- DRM or access control: prevent unauthorized playback.\n- Watermark: identify a copy after distribution.\n- Records: contracts, manifests and timestamps for the investigation.",
    limits: [
      "No encryption, playback control or revocation.",
      "Does not prevent copying.",
      "Use it after the fact, alongside access controls."
    ],
    faq: [
      {
        q: "Does Aureal prevent unauthorized playback?",
        a: "No. It is not DRM and does not encrypt, block or revoke playback. It records an ID that can be recovered from a copy."
      },
      {
        q: "Should I stop using DRM?",
        a: "No. Use DRM for access control and Aureal for after-the-fact leak investigation."
      }
    ],
    related: ["what-is-audio-watermarking", "audio-watermark-vs-metadata", "audio-watermark-vs-fingerprinting"]
  },
  {
    slug: "payload-id-and-key",
    h1: "The 32-bit payload and the key: how Aureal IDs work",
    shortTitle: "Payload ID and key",
    category: "Basics",
    metaDesc:
      "Aureal carries a 32-bit ID plus CRC. Detection needs the same key. Re-marking with the same key and a new ID can destroy the original.",
    shortAnswer:
      "The 32-bit ID is a lightweight pointer, not a metadata container. Detection needs the same key used for embedding. Re-marking the same audio with the same key and a conflicting ID can destroy the original ID, so use a fresh copy per recipient, a private key and a manifest.",
    technicalBasis: "32-bit payload, keyed PN sequences, ~1 s frame",
    detail:
      "Aureal packs a 32-bit payload plus a 16-bit CRC into 48 BPSK symbols per frame. The spreading sequence is derived from the key, so detection without that key is not the supported path. The built-in default key is a public constant for demos; production use should generate a private key. Keep the key out of the distributed audio and in your own records.",
    trapExample:
      "Re-mark the same master with the same key and a new ID for each recipient.",
    whyItFails:
      "Same-key overwrite collisions cause destructive bit interference. Reusing one key with conflicting IDs on the same master is not a safe way to issue per-recipient copies, and losing the key can make recovery impossible.",
    redlineExample:
      "Use a private key and a fresh ID per distribution copy:\n- Generate a unique 32-bit ID for each recipient.\n- Keep the key and the recipient manifest under your control.\n- Do not re-mark the same master with the same key and a conflicting ID.",
    limits: [
      "Detection requires the same key used for embedding.",
      "The built-in default key is public; use a private key.",
      "Same-key overwrite with a conflicting ID can destroy the original ID."
    ],
    faq: [
      {
        q: "How many IDs are available?",
        a: "The payload is 32 bits, so there are 4,294,967,296 possible IDs. Treat the ID as a pointer and keep a manifest; collisions and reuse are an operational risk."
      },
      {
        q: "What happens if I lose the key?",
        a: "You may not be able to recover the ID reliably. Keep the key with the distribution records and use a private key, not the public default."
      }
    ],
    related: ["what-is-audio-watermarking", "audio-watermark-vs-fingerprinting", "mp3-128kbps"]
  },
  {
    slug: "mp3-128kbps",
    h1: "Does an audio watermark survive MP3 128 kbps?",
    shortTitle: "MP3 128 kbps",
    category: "Robustness",
    metaDesc:
      "MP3 128 kbps passed on Aureal's local fixture matrix. That is a fixture result, not a guarantee for every encoder, source or platform.",
    shortAnswer:
      "In Aureal's local fixture matrix — 20 seconds, 44.1 kHz, strength 0.5 — MP3 128 kbps passed for the high, mid and dual bands. That is evidence for the named fixtures, not a guarantee for every encoder, source or platform. Test the actual delivery chain.",
    technicalBasis: "Local fixture matrix: 20 s, 44.1 kHz, strength 0.5, MP3 128 kbps",
    detail:
      "The named fixture is a 20 s, 44.1 kHz, strength-0.5 test with a speech-like source and a local ffmpeg encode. It is a sanity check, not a service-level guarantee. For a release, encode a representative master, scan it, and record the carrier band, confidence and BER. If the result is marginal, adjust the band or strength before distribution.",
    trapExample:
      "It survived MP3 128 once, so it survives every encoder.",
    whyItFails:
      "One fixture result does not cover other encoders, bitrates, source material or platform processing. The published table is evidence for the named fixtures, not a universal compatibility claim.",
    redlineExample:
      "Test the exact chain before shipping:\nffmpeg -i master.wav -codec:a libmp3lame -b:a 128k test.mp3\nnode aureal-watermark.cjs detect test.mp3 --id 883921\nRecord the band, confidence and BER with the release.",
    limits: [
      "Fixture result: 20 s, 44.1 kHz, strength 0.5, named encoders.",
      "Not a guarantee for every source, encoder or platform.",
      "Test the actual delivery chain and record band/confidence/BER."
    ],
    faq: [
      {
        q: "Which carrier band should I use for MP3?",
        a: "All three bands passed the local 128 kbps fixture. For lossy distribution, dual or mid band is a practical starting point; validate on your own material before committing."
      },
      {
        q: "Is 128 kbps guaranteed?",
        a: "No. The fixture result is evidence for the named setup, not a guarantee for every encoder, source or platform."
      }
    ],
    related: ["mp3-320kbps", "aac-transcoding", "opus-transcoding"]
  },
  {
    slug: "mp3-320kbps",
    h1: "Does an audio watermark survive MP3 320 kbps?",
    shortTitle: "MP3 320 kbps",
    category: "Robustness",
    metaDesc:
      "MP3 320 kbps passed on Aureal's local fixture matrix. Higher bitrate helps, but recovery still depends on source, encoder and processing.",
    shortAnswer:
      "MP3 320 kbps passed on the same local fixture matrix for all three bands. A higher bitrate reduces codec damage, but it does not make recovery universal; source material, encoder settings and later processing still matter.",
    technicalBasis: "Local fixture matrix: 20 s, 44.1 kHz, strength 0.5, MP3 320 kbps",
    detail:
      "At 320 kbps the codec has more bits to preserve the signal, but the watermark still competes with the source. A quiet or narrow-band master may behave differently from the fixture. Test the actual encoder and the platform's transcode; a high bitrate upstream does not guarantee what a streaming service does downstream.",
    trapExample:
      "320 kbps is transparent, so there is no need to test the watermark.",
    whyItFails:
      "Higher bitrate reduces codec damage but does not make the result universal. The encoder, source material and any later processing still affect recovery.",
    redlineExample:
      "Run the same test at the bitrate you distribute:\nffmpeg -i master.wav -codec:a libmp3lame -b:a 320k test.mp3\nnode aureal-watermark.cjs detect test.mp3 --id 883921\nDo not infer 320 kbps survival from a 128 kbps result, or the reverse.",
    limits: [
      "Fixture result: 20 s, 44.1 kHz, strength 0.5, named encoders.",
      "Higher bitrate reduces codec damage but is not a guarantee.",
      "Test the actual delivery chain and record the result."
    ],
    faq: [
      {
        q: "Is 320 kbps safer than 128 kbps?",
        a: "It reduces codec damage in the fixture matrix, but it is not a guarantee. Test the actual encoder and the rest of the delivery chain."
      },
      {
        q: "Do I need to test if I only use 320?",
        a: "Yes. Source material, encoder settings and later processing can still affect recovery."
      }
    ],
    related: ["mp3-128kbps", "aac-transcoding", "resampling"]
  },
  {
    slug: "aac-transcoding",
    h1: "Does an audio watermark survive AAC transcoding?",
    shortTitle: "AAC transcoding",
    category: "Robustness",
    metaDesc:
      "AAC 128 kbps passed on Aureal's local fixture matrix. AAC encoders vary; validate the exact bitrate and encoder you use.",
    shortAnswer:
      "AAC 128 kbps passed on Aureal's local fixture matrix. AAC is a family of encoders and bitrates, so the result does not transfer automatically to every AAC implementation. Validate the exact target encoder and keep the result with the distribution record.",
    technicalBasis: "Local fixture matrix: AAC 128 kbps",
    detail:
      "AAC is used by many platforms and encoders, and the fixture covers one local encoder at 128 kbps. The codec's psychoacoustic model can remove low-level content, especially at low bitrates. Use dual or mid band as a starting point, and test the exact encoder and bitrate your platform uses.",
    trapExample:
      "AAC and MP3 behave the same, so one test covers both.",
    whyItFails:
      "AAC is a family of encoders and bitrates; a result at 128 kbps on one encoder does not transfer to every AAC implementation.",
    redlineExample:
      "Test the target AAC encoder:\nffmpeg -i master.wav -codec:a aac -b:a 128k test.m4a\nnode aureal-watermark.cjs detect test.m4a --id 883921\nRepeat with the encoder and bitrate your platform actually uses.",
    limits: [
      "Fixture result: AAC 128 kbps on a named local encoder.",
      "AAC encoders and bitrates vary.",
      "Validate the exact target pipeline."
    ],
    faq: [
      {
        q: "Does AAC 128 kbps always work?",
        a: "No. The local fixture passed, but AAC encoders and bitrates vary. Validate the exact encoder and bitrate you distribute."
      },
      {
        q: "What about AAC 256 kbps?",
        a: "The documented tested range includes AAC 64–256 kbps in local fixtures. A pass at one setting is not a universal guarantee; test your target."
      }
    ],
    related: ["mp3-128kbps", "mp3-320kbps", "opus-transcoding"]
  },
  {
    slug: "opus-transcoding",
    h1: "Does an audio watermark survive Opus transcoding?",
    shortTitle: "Opus transcoding",
    category: "Robustness",
    metaDesc:
      "Opus is in Aureal's tested codec family. There is no separate per-bitrate guarantee; test the Opus setting and encoder you actually use.",
    shortAnswer:
      "Opus is in the codec family Aureal has tested locally (Opus, WebM and Ogg Vorbis). There is no separate published per-bitrate table for every Opus setting, so a transparent-sounding encode is not a recovery guarantee. Test the bitrate and encoder you use.",
    technicalBasis: "Tested codec family: Opus, WebM, Ogg Vorbis",
    detail:
      "Opus is often used at 48 kHz and low latency; its encoder can discard content it considers inaudible. Aureal's local fixtures include Opus, WebM and Ogg Vorbis, but there is no per-bitrate guarantee. A 96 kbps test is a data point, not a rule for 64 kbps or 128 kbps.",
    trapExample:
      "Opus at 96 kbps is transparent, so the watermark is guaranteed to survive.",
    whyItFails:
      "Opus is in the tested codec family, but no separate per-bitrate table is published. A transparent encode at one setting is not evidence for every setting.",
    redlineExample:
      "Test the actual Opus setting:\nffmpeg -i master.wav -codec:a libopus -b:a 96k test.opus\nnode aureal-watermark.cjs detect test.opus --id 883921\nRecord the result; do not treat one bitrate as universal.",
    limits: [
      "Opus is in the tested codec family; no separate per-bitrate guarantee.",
      "Encoder settings and source material vary.",
      "Validate the exact target pipeline."
    ],
    faq: [
      {
        q: "Is Opus supported?",
        a: "Opus is in the tested codec family (Opus, WebM, Ogg Vorbis) on local fixtures. There is no separate per-bitrate table, so test the setting you use."
      },
      {
        q: "Does a transparent Opus encode guarantee recovery?",
        a: "No. Transparency and watermark recovery are different properties; test the actual file."
      }
    ],
    related: ["mp3-128kbps", "aac-transcoding", "resampling"]
  },
  {
    slug: "resampling",
    h1: "Does an audio watermark survive resampling?",
    shortTitle: "Resampling",
    category: "Robustness",
    metaDesc:
      "Aureal retries detection at 44.1 kHz and 48 kHz after the native rate. Other rates need their own test; resampling is not a universal guarantee.",
    shortAnswer:
      "Aureal first tries detection at the file's native sample rate. If that fails, it retries at 44.1 kHz and 48 kHz using a 64-phase windowed-sinc resampler. Those are the only automatic retry rates; unusual rates and third-party resamplers need their own test.",
    technicalBasis: "64-phase windowed-sinc resampler; retry at 44.1/48 kHz",
    detail:
      "Aureal's detector normalizes to 44.1 kHz and 48 kHz only when the native-rate attempt fails. That covers common delivery rates, but not 96 kHz, 32 kHz or unusual broadcast rates. If your pipeline uses a different rate, test the exact resampler and keep the result with the release; do not assume the two-rate retry covers it.",
    trapExample:
      "Any sample rate works because the detector resamples everything.",
    whyItFails:
      "Only 44.1 kHz and 48 kHz are automatic retry targets, and the detector tries the native rate first. An unusual rate may still work, but it is not covered by the documented retry path.",
    redlineExample:
      "Keep delivery at 44.1 kHz or 48 kHz when you can, and test the exception:\nffmpeg -i copy.wav -ar 48000 copy-48k.wav\nnode aureal-watermark.cjs detect copy-48k.wav --id 883921\nFor unusual rates, test the exact resampler and record the result.",
    limits: [
      "Automatic retry covers 44.1 kHz and 48 kHz only.",
      "Native-rate detection is attempted first.",
      "Unusual rates and third-party resamplers need their own test."
    ],
    faq: [
      {
        q: "Does Aureal resample any input?",
        a: "It first tries the native sample rate, then retries at 44.1 kHz and 48 kHz if needed. It does not automatically try every rate."
      },
      {
        q: "Will a 96 kHz file work?",
        a: "It may, but it is not covered by the documented two-rate retry. Test the exact file and record the result."
      }
    ],
    related: ["mp3-128kbps", "mp3-320kbps", "aac-transcoding"]
  },
  {
    slug: "cropping-and-trimming",
    h1: "Does an audio watermark survive cropping?",
    shortTitle: "Cropping and trimming",
    category: "Robustness",
    metaDesc:
      "Aureal locks onto a keyed chirp preamble, so unaligned crops can decode. On the named 12 s fixture, 1.8 s excerpts recovered at half the offsets.",
    shortAnswer:
      "Aureal does not need the file to start at a frame boundary. The detector searches for a key-derived chirp preamble and folds partial frames, so a clip cut from the middle can still decode. On the named 12 s fixture, 1.8 s excerpts recovered at 49 of 93 start offsets, and anything shorter than one frame (about 1 s at 44.1 kHz) never decoded.",
    technicalBasis: "Keyed IQ chirp preamble search; circular frame folding; ~1 s frame",
    detail:
      "The payload repeats in frames of about one second. Instead of assuming the first sample is a frame start, detection cross-correlates the key-derived chirp preamble to find the sub-sample offset, then folds the available repetitions. For clips between roughly 1 s and 2 s, circular modulo folding lets fragments on both sides of a cut contribute to the same 48-symbol codeword. The mechanics explain why a mid-track cut is not automatically fatal, but they do not make short clips reliable: the offset sweep in the fixture still fails about half the time at 1.8 s, and a second encode on top reduces the margin further.",
    trapExample:
      "Cutting the first five seconds removes the watermark, because the file no longer starts at frame zero.",
    whyItFails:
      "The detector does not require a frame-aligned start. It searches for the preamble and recovers the frame phase from the audio itself. A five-second clip usually contains several frames, so the start offset is not the problem; total duration, codec damage and the signal-to-noise ratio in the carrier band are.",
    redlineExample:
      "Test the crop you actually plan to distribute, at more than one offset:\nffmpeg -ss 3.0 -t 1.8 -i marked.wav -c copy clip-a.wav\nnode aureal-watermark.cjs detect clip-a.wav --id 883921\nffmpeg -ss 7.4 -t 1.8 -i marked.wav -c copy clip-b.wav\nnode aureal-watermark.cjs detect clip-b.wav --id 883921\nRecord duration, offset and result; one passing offset is not a guarantee for the clip.",
    limits: [
      "Anything shorter than one frame (~1 s at 44.1 kHz) is never recovered.",
      "At 1.8 s, about half of the start offsets recovered on the named 12 s fixture (49/93).",
      "Result depends on source audio, codec and the offset of the cut.",
      "The reported carrier band can be wrong after filtering."
    ],
    faq: [
      {
        q: "How short can a clip be and still decode?",
        a: "A complete frame is the floor: about 1 s at 44.1 kHz. Even around 1.8 s, recovery was partial and depended on the start offset in the local fixture. Test the exact clip rather than assuming a duration is safe."
      },
      {
        q: "Does trimming the intro remove the watermark?",
        a: "Not by itself. The detector finds the frame phase from the preamble, so a mid-track cut can decode. If the remaining clip is too short or has been re-encoded, recovery can still fail."
      }
    ],
    related: ["short-excerpts", "speed-drift", "resampling"]
  },
  {
    slug: "speed-drift",
    h1: "Does a watermark survive speed and pitch changes?",
    shortTitle: "Speed and pitch drift",
    category: "Robustness",
    metaDesc:
      "Aureal retries detection at ±0.2%, ±0.5% and ±1.0% speed drift. That covers small linear changes, not tape wow, varispeed edits or large tempo shifts.",
    shortAnswer:
      "A small linear speed change stretches the audio and shifts the watermark's carrier. If native detection fails, Aureal re-runs it at six factors between 0.99× and 1.01× (about ±1.0%). The documented fixtures cover that range; larger tempo changes, varispeed edits and analog wow and flutter need their own test.",
    technicalBasis: "Drift rake sweep at 0.998/1.002, 0.995/1.005 and 0.99/1.01",
    detail:
      "A speed change is a resampling operation: the duration, the pitch and every carrier frequency move by the same factor. The detector therefore compensates by resampling again and scoring the result, rather than trying to stretch the correlation window. The sweep runs only after the native-rate attempt and the 44.1/48 kHz retry both fail, and it stops at the first factor that decodes; when it succeeds, the result reports the factor it used. The grid is coarse by design. A 2% change is outside it, and repeated small edits can accumulate past the range even if each step looks harmless.",
    trapExample:
      "A 4% speed-up is safe because the detector compensates for speed changes.",
    whyItFails:
      "The documented sweep covers about ±1.0% in fixed steps (0.99× to 1.01×). A 4% change moves the carriers well beyond that grid, so the detector has no tested hypothesis for it. Analog wow and flutter is also continuously varying, not a single fixed factor.",
    redlineExample:
      "Create a 1% speed-and-pitch change and test it; then test a change outside the sweep to see the boundary:\nffmpeg -i marked.wav -filter:a \"asetrate=44100*1.01,aresample=44100\" speed-101.wav\nnode aureal-watermark.cjs detect speed-101.wav --id 883921\nffmpeg -i marked.wav -filter:a \"asetrate=44100*1.04,aresample=44100\" speed-104.wav\nnode aureal-watermark.cjs detect speed-104.wav --id 883921\nA pass at 1% does not imply a pass at 4%.",
    limits: [
      "Documented drift coverage is about ±1.0% in fixed steps.",
      "No general guarantee for analog wow and flutter.",
      "Repeated edits can accumulate beyond the tested range.",
      "The sweep is a local fixture result, not a per-file guarantee."
    ],
    faq: [
      {
        q: "Does a DJ's 3% pitch shift work?",
        a: "It is outside the documented ±1.0% sweep. It may still decode with enough signal and frames, but it is not covered; test the exact edit before relying on it."
      },
      {
        q: "Can I see which drift factor was applied?",
        a: "Yes. When the drift sweep succeeds, the detection result includes the factor it used. That is useful for reproducing a marginal result, not for proving who edited the file."
      }
    ],
    related: ["resampling", "cropping-and-trimming", "mp3-128kbps"]
  },
  {
    slug: "stereo-downmix",
    h1: "Does the watermark survive a stereo-to-mono downmix?",
    shortTitle: "Stereo to mono downmix",
    category: "Robustness",
    metaDesc:
      "Aureal embeds in the mid component, so an in-phase mono downmix adds about 3 dB. Out-of-phase content and widening can still reduce or cancel it.",
    shortAnswer:
      "Aureal spreads the watermark across the mid and side components. Because an in-phase stereo-to-mono downmix sums the channels, the mid layer gains roughly 3 dB of processing gain; the local phase-inversion fixture also decoded. Out-of-phase mixes, aggressive stereo widening and polarity tricks can still reduce or cancel a layer, so test the actual master rather than assuming mono is safe.",
    technicalBasis: "Mid/side embedding; detector scores mono sum and side difference",
    detail:
      "For stereo input the embedder derives a mid watermark and a side watermark with separate key streams, then writes left and right as (mid + side)/√2 and (mid − side)/√2. Detection can score both the channel sum (mid) and the difference (side), which is why a simple polarity inversion does not automatically erase both layers. A standard L+R fold-down preserves the mid layer with about +3 dB; an L−R extraction preserves the side layer instead. The weak case is a mix where the two channels are substantially out of phase before the fold-down, because the sum can attenuate the very component that carries the mark.",
    trapExample:
      "Mono downmix is mathematically immune, so every mono file will decode.",
    whyItFails:
      "The +3 dB figure is processing gain for an in-phase sum, not immunity. If the channels are out of phase around the carrier, summing them attenuates the watermark, and heavy widening or all-pass processing can decorrelate the layers. The local phase-inversion fixture is evidence for that fixture, not a blanket guarantee.",
    redlineExample:
      "Test the exact fold-down, including a deliberately out-of-phase variant:\nffmpeg -i marked.wav -ac 1 mono.wav\nnode aureal-watermark.cjs detect mono.wav --id 883921\nffmpeg -i marked.wav -af \"pan=mono|c0=0.5*c0-0.5*c1\" side-only.wav\nnode aureal-watermark.cjs detect side-only.wav --id 883921\nThe first should favour the mid layer, the second the side layer. Keep the result with the master.",
    limits: [
      "In-phase downmix adds about +3 dB, not immunity.",
      "Out-of-phase content and stereo widening can attenuate a layer.",
      "The phase-inversion result is a local synthetic fixture.",
      "Detection scores the mono sum and the side difference, not every possible matrix."
    ],
    faq: [
      {
        q: "Does distributing in mono hurt recovery?",
        a: "An in-phase L+R fold-down preserves the mid layer and adds about 3 dB of processing gain in the current detector. A mix with substantial out-of-phase energy is the case to test."
      },
      {
        q: "What about a vocal remover that subtracts the channels?",
        a: "An L−R extraction removes the mid layer and leaves the side layer. The detector can score the difference, and the local phase-inversion fixture decoded, but treat that as a fixture result and test your processing chain."
      }
    ],
    related: ["clipping-and-limiting", "background-noise", "audio-watermark-vs-metadata"]
  },
  {
    slug: "clipping-and-limiting",
    h1: "Does clipping or limiting destroy the watermark?",
    shortTitle: "Clipping and limiting",
    category: "Robustness",
    metaDesc:
      "Hard clipping at +12 dB passed a local fixture, as did one heavy limiter. Both are nonlinear; test the final master at its release loudness.",
    shortAnswer:
      "In a local synthetic fixture, +12 dB of hard clipping did not corrupt the symbol signs, and one heavy compressor and limiter fixture still decoded. That is not a guarantee. Clipping is nonlinear, aggressive limiting changes gain over time, and a low-strength watermark has less margin; test the master after the loudness stage, not before it.",
    technicalBasis: "+12 dB hard-clip fixture; one heavy limiter fixture; embed limiter ≤0.995",
    detail:
      "The embedder has its own sample-peak limiter, but it is not a mastering limiter: it engages only when the mixed output exceeds 0.999 and then scales the whole buffer so the peak is at most 0.995. That keeps the embedding step from introducing clipping. What happens later is a different problem. Hard clipping flattens peaks and adds broadband harmonics; a look-ahead limiter applies time-varying gain that can pump a low-level watermark. The +12 dB fixture is evidence that the symbol polarity can survive severe distortion, not evidence that every limiter preserves the code.",
    trapExample:
      "If it survives +12 dB of clipping, any limiter setting is fine.",
    whyItFails:
      "The fixture covers one hard-clip case and one compressor case on local material. A limiter's release time, look-ahead and gain reduction interact with the host signal, and the watermark sits far below the program level. A different master, strength or limiter can produce a different result.",
    redlineExample:
      "Test the release master after limiting, at the loudness you will ship:\nffmpeg -i marked.wav -af \"alimiter=limit=0.9:attack=5:release=50\" limited.wav\nnode aureal-watermark.cjs detect limited.wav --id 883921\nffmpeg -i marked.wav -af \"volume=12dB\" -c:a pcm_s16le clipped.wav\nnode aureal-watermark.cjs detect clipped.wav --id 883921\nIf the result is marginal, lower the limiter gain reduction or raise the embedding strength before distribution.",
    limits: [
      "The +12 dB clipping and heavy-limiter results are local fixtures.",
      "Clipping is nonlinear and can add broadband distortion.",
      "A limiter's time-varying gain can reduce the watermark margin.",
      "Test after the final loudness stage, not on the pre-master."
    ],
    faq: [
      {
        q: "Does the embedder clip the audio?",
        a: "It includes a sample-peak limiter that engages only above 0.999 and scales the whole buffer to a peak of at most 0.995. It does not replace a mastering limiter, and it does not protect the mark from later clipping."
      },
      {
        q: "Is clipping safer than limiting, or the reverse?",
        a: "There is no general ordering. Both are nonlinear and programme-dependent; the only useful answer is a test on the final master at its release settings."
      }
    ],
    related: ["background-noise", "mp3-128kbps", "stereo-downmix"]
  },
  {
    slug: "background-noise",
    h1: "Does the watermark survive background noise?",
    shortTitle: "Background noise",
    category: "Robustness",
    metaDesc:
      "Noise fixtures passed down to −30 dBFS. In-band SNR decides, broadband noise is a weak host, and near-silence is muted rather than marked.",
    shortAnswer:
      "The local fixture matrix records additive noise at −30 dBFS for all three bands, and the README's adversarial table names a −20 dB pink-noise fixture. Both are local results. Recovery depends on the signal-to-noise ratio inside the carrier band, not only on the file's overall loudness; broadband noise and 8 kHz paths are weak hosts.",
    technicalBasis: "Additive noise fixtures: −30 dBFS matrix, −20 dB pink noise",
    detail:
      "Spread-spectrum detection integrates repeated frames, so noise that looks severe on a waveform can still leave a decodable mark when enough frames are available. The relevant quantity is the in-band SNR at the carrier, which is why a quiet recording with low self-noise can outperform a loud one with broadband hiss. The embedder also mutes the watermark in near-silence (at or below about −54 dBFS) to keep pauses clean, so a clip made only of room tone may carry no mark at all, regardless of its duration.",
    trapExample:
      "A −20 dB noise floor is fine because the fixture passed at −30 dBFS.",
    whyItFails:
      "Overall dBFS and in-band SNR are different measurements. Pink noise concentrates energy at low frequencies, while the carrier sits in the mid or high band; the result also depends on how many frames the file provides and on any codec that follows. A level that passed on one fixture says little about a different noise spectrum.",
    redlineExample:
      "Measure the result at two noise levels on the actual master, and keep the carrier band in the record:\nffmpeg -i marked.wav -f lavfi -i \"anoisesrc=color=pink:amplitude=0.01\" -filter_complex \"[0:a][1:a]amix=inputs=2:duration=first\" noisy.wav\nnode aureal-watermark.cjs detect noisy.wav --id 883921\nRepeat at a higher amplitude and compare confidence and BER rather than a single pass/fail.",
    limits: [
      "The noise results are local fixtures, not platform guarantees.",
      "Broadband noise is a known weak host.",
      "The embedder mutes the watermark in near-silence (≤ about −54 dBFS).",
      "Codec damage after the noise compounds the loss."
    ],
    faq: [
      {
        q: "What SNR do I need?",
        a: "There is no single number that holds across content. Detection integrates repeated frames, and the useful margin is the in-band SNR at the carrier. Test the actual noise spectrum and codec, and record confidence and BER."
      },
      {
        q: "Does noise erase the ID?",
        a: "It raises the bit error rate and can push a marginal file below the detection threshold. With enough frames and a strong enough mark it may still decode; the fixture results show the direction, not a guarantee."
      }
    ],
    related: ["8khz-telephony", "clipping-and-limiting", "mp3-128kbps"]
  },
  {
    slug: "8khz-telephony",
    h1: "Does the watermark work at 8 kHz?",
    shortTitle: "8 kHz telephony",
    category: "Robustness",
    metaDesc:
      "At 8 kHz the standard carriers do not fit under Nyquist, so Aureal uses a ~2–4 kHz fallback. It is a known weak case; test before relying on it.",
    shortAnswer:
      "Aureal accepts audio from 8 kHz, but the standard 4.5–13.5 kHz and 17–19.5 kHz carriers cannot exist under an 8 kHz Nyquist limit. The embedder therefore uses a single fallback band inside the available spectrum (roughly 2.1–3.9 kHz), a weak speech-band host. A 44.1 kHz marked file downsampled to 8 kHz loses both standard carriers; do not expect recovery from that path.",
    technicalBasis: "Nyquist-safe fallback band at 8 kHz; standard presets need higher rates",
    detail:
      "At an 8 kHz sample rate the usable spectrum ends at 4 kHz. The standard mid and high presets start at 4.5 kHz and 17 kHz, so neither fits; the embedder chooses one wide band under Nyquist instead, which places the mark in the speech region where codecs and noise do the most damage. The same limit applies in reverse: if you mark a 44.1 kHz master and then downsample it for a telephone or voice-assistant path, the resampler removes both standard carriers before the file reaches the far end. Upsampling the 8 kHz file later cannot restore the removed band.",
    trapExample:
      "Downsample the marked promo to 8 kHz for the phone and the ID will still trace the call.",
    whyItFails:
      "A true 8 kHz downsample discards everything above 4 kHz, which includes both standard carrier bands. Only a file embedded after the 8 kHz conversion uses the fallback band, and even that path is a documented weak case once a telephony codec and network noise are applied.",
    redlineExample:
      "Compare the two paths instead of assuming they are equivalent:\nffmpeg -i marked-44k.wav -ar 8000 -ac 1 downsampled.wav\nnode aureal-watermark.cjs detect downsampled.wav --id 883921\nffmpeg -i master.wav -ar 8000 -ac 1 master-8k.wav\nnode aureal-watermark.cjs embed master-8k.wav marked-8k.wav --id 883921\nnode aureal-watermark.cjs detect marked-8k.wav --id 883921\nThen test the actual telephony codec; a pass on the WAV is not a pass on the call.",
    limits: [
      "Standard mid and high carriers do not fit at 8 kHz.",
      "Only the fallback band (roughly 2.1–3.9 kHz) is available at 8 kHz.",
      "Downsampling a 44.1 kHz marked file destroys both standard carriers.",
      "8 kHz is a documented weak case; telephony codecs make it weaker."
    ],
    faq: [
      {
        q: "Will a phone recording decode?",
        a: "Only if the mark was embedded at 8 kHz with the fallback band and survived the telephony codec and noise. That is a weak, test-every-time path, not a guarantee. A downsampled 44.1 kHz mark should be treated as gone."
      },
      {
        q: "Can I upsample the 8 kHz file back to 44.1 kHz?",
        a: "You can, but resampling cannot recreate energy that was removed. The standard carriers are still absent; the file remains an 8 kHz-bandwidth recording for watermark purposes."
      }
    ],
    related: ["resampling", "short-excerpts", "background-noise"]
  },
  {
    slug: "short-excerpts",
    h1: "How short a clip can still be detected?",
    shortTitle: "Short excerpts",
    category: "Robustness",
    metaDesc:
      "A frame is about one second. Sub-1 s excerpts never decoded; at 1.0–1.8 s recovery was partial and depended on the start offset in the named fixture.",
    shortAnswer:
      "Aureal repeats a frame about once per second, and detection needs enough of that frame to rebuild the codeword. On the named 12 s fixture, 1.8 s excerpts recovered at 49 of 93 start offsets and 1.0 s excerpts at about half; anything shorter than one frame (about 1 s at 44.1 kHz) never decoded. A short clip is not reliably traceable.",
    technicalBasis: "~1 s repeated frame; circular folding; 49/93 offsets at 1.8 s",
    detail:
      "More repetitions generally mean more integration and a steadier decision, which is why a full track is easier than a snippet. The circular folding path is what gives sub-2-second crops a chance: fragments on either side of a frame boundary are wrapped into their symbol slots instead of being discarded. That helps only when a complete 48-symbol codeword can be assembled. Below one frame there is not enough material, and around 1.8 s the result depends on where the cut falls, so an offset sweep is the honest test.",
    trapExample:
      "A 0.5 s snippet is enough to support a leak claim.",
    whyItFails:
      "Half a second is below one frame at 44.1 kHz, so there is no complete codeword to decode. Even at 1.8 s the local fixture recovered only about half the offsets, and a lossy encode on top can remove the marginal cases.",
    redlineExample:
      "Sweep the offset instead of testing one lucky clip:\nffmpeg -ss 3.0 -t 1.8 -i marked.wav -c copy clip-a.wav\nffmpeg -ss 7.4 -t 1.8 -i marked.wav -c copy clip-b.wav\nnode aureal-watermark.cjs detect clip-a.wav --id 883921\nnode aureal-watermark.cjs detect clip-b.wav --id 883921\nRecord each duration and offset. A single pass is not evidence that short clips are reliable.",
    limits: [
      "Anything shorter than one frame (~1 s at 44.1 kHz) is never recovered.",
      "At 1.8 s, about half the offsets recovered on the named fixture (49/93).",
      "The result depends on the start offset, source audio and codec.",
      "More frames help, but do not guarantee a specific clip."
    ],
    faq: [
      {
        q: "Can I detect a 5 s clip?",
        a: "Usually there are several frames to integrate, so the odds are better than at 1.8 s. It is still a test on the actual file: codec, noise and the cut offset all matter."
      },
      {
        q: "Why did a 2 s clip fail when a 1.8 s clip passed?",
        a: "Recovery depends on which fragments of the codeword the cut leaves and on the local signal, not on duration alone. The fixture's 49/93 result is a warning against treating any short duration as safe."
      }
    ],
    related: ["cropping-and-trimming", "8khz-telephony", "resampling"]
  },
  {
    slug: "hmac-seal-vs-signature",
    h1: "Is the report seal a digital signature?",
    shortTitle: "HMAC seal vs. signature",
    category: "Forensics",
    metaDesc:
      "No. Aureal seals reports with HMAC-SHA256, a shared-secret MAC. It detects tampering and shows key possession, not identity, authorship or time.",
    shortAnswer:
      "The forensic report carries an HMAC-SHA256 seal over its canonical JSON body. It is a shared-secret message authentication code, not a public-key signature: anyone with the same key can create or verify it, so it cannot by itself prove who produced the report or when. If you omit the key, the built-in default secret is public; use the --key option and protect that secret.",
    technicalBasis: "HMAC-SHA256 over sorted-keys JSON; shared secret; public default",
    detail:
      "The seal is computed over the report body with the signature object removed, using sorted-keys JSON canonicalisation, and the comparison uses a constant-time equality check. The signature object records the algorithm, the canonicalisation version and what it covers. That design makes any edit to a covered field detectable, including the verdict, payload ID, metrics and file hashes. It does not create asymmetry: the same secret both creates and checks the seal, so a holder can re-seal a modified report. The built-in default secret is a fixed public constant; a report sealed with it can be reproduced by anyone, so it cannot evidence possession of a private key.",
    trapExample:
      "The HMAC seal is a cryptographic signature that proves the label produced the report.",
    whyItFails:
      "An HMAC is symmetric. It proves that someone with the secret produced the seal and that the covered bytes have not changed, not who that someone was. With the public default, even the first part is weak. Identity needs a public-key signature or an independent chain of custody, neither of which the seal provides.",
    redlineExample:
      "Seal with a private secret, verify with the same secret, and store the key separately from the report:\nnode aureal-watermark.cjs detect evidence.mp3 --id 883921 --report proof.json --key \"$AUREAL_SEAL_KEY\"\nnode aureal-watermark.cjs verify-report proof.json --key \"$AUREAL_SEAL_KEY\"\n# exit 0 = body unchanged and same key; 1 = modified or wrong key\nThe report does not identify its author, and the sealed timestamp is self-asserted.",
    limits: [
      "Shared secret, not a public-key signature.",
      "The built-in default secret is public; use the --key option.",
      "A valid seal does not establish authorship, ownership or scan time.",
      "Anyone holding the key can create a valid seal over a different report."
    ],
    faq: [
      {
        q: "Can I publish the report and the key together?",
        a: "You can, but then anyone can re-seal a modified report and the seal stops distinguishing your version. Publish the report; keep the key with the party that is supposed to verify it."
      },
      {
        q: "Does a valid seal prove the scan happened?",
        a: "It proves the covered fields match the secret. The timestamp is written by the generating machine and is covered by the seal, but nothing independently attests that the clock was correct or that the scan occurred at that moment."
      }
    ],
    related: ["verify-forensic-report", "sha-256-sha-512", "what-is-audio-watermarking"]
  },
  {
    slug: "verify-forensic-report",
    h1: "How do I verify a forensic report?",
    shortTitle: "Verify a forensic report",
    category: "Forensics",
    metaDesc:
      "Run verify-report with the same shared secret. Exit 0 means the body is unchanged; exit 1 means modified or wrong key. It does not prove authorship.",
    shortAnswer:
      "Run auralwatermark verify-report proof.json --key <secret>. It recomputes the HMAC over the report body and compares it in constant time. Exit code 0 means the seal matches; 1 means the report changed or the key is wrong. Verification needs the same shared secret that sealed the report, and it checks the report, not the authorship of the audio.",
    technicalBasis: "verify-report CLI; HMAC-SHA256; exit 0/1",
    detail:
      "The verifier reads the JSON, reconstructs the canonical body with the signature removed, recomputes HMAC-SHA256 and compares the result. A single changed character in a covered field, or a different secret, produces exit 1. If the report predates the current canonicalisation or carries an unknown scheme, verification fails with an explicit reason instead of guessing. The seal covers the recorded file hashes, so they cannot be swapped without breaking it, but verification does not re-read the audio: check the file's SHA-256 and SHA-512 yourself if the evidence file matters.",
    trapExample:
      "A successful verify-report proves the audio is authentic and the report is conclusive.",
    whyItFails:
      "The command checks a shared-secret seal over the report body. It does not hash the audio file again, does not establish who ran the scan, and does not provide a trusted timestamp or chain of custody. Re-hash the evidence file and compare it with the recorded values before drawing any conclusion about the file.",
    redlineExample:
      "Verify the seal, then verify the evidence file independently:\nnode aureal-watermark.cjs verify-report proof.json --key \"$AUREAL_SEAL_KEY\"\n# exit 0 = seal valid; 1 = modified or wrong key\nsha256sum evidence.mp3\n# compare with targetFile.hashes.sha256 in proof.json\nUse the JSON for the full SHA-512; the text certificate shortens it for display.",
    limits: [
      "Requires the same shared secret used to seal the report.",
      "A valid seal does not establish authorship, ownership or scan time.",
      "The command does not re-hash the audio file; compare the recorded hashes yourself.",
      "The public default key makes the seal reproducible by anyone."
    ],
    faq: [
      {
        q: "What if I lost the key?",
        a: "You cannot verify the existing seal. You can regenerate a report with a new key, but that is a new seal, not a recovery of the old one. Keep the key with the verification record."
      },
      {
        q: "Can I verify with the default key?",
        a: "Yes, if the report was sealed with the default. That confirms the body matches the public constant, which anyone can reproduce; it is not evidence that a particular party produced the report."
      }
    ],
    related: ["hmac-seal-vs-signature", "sha-256-sha-512", "payload-id-and-key"]
  },
  {
    slug: "sha-256-sha-512",
    h1: "What do the report's SHA-256 and SHA-512 hashes prove?",
    shortTitle: "SHA-256 and SHA-512",
    category: "Forensics",
    metaDesc:
      "They fingerprint the scanned file's bytes. A match shows the file is unchanged; it does not establish authorship, ownership or creation time.",
    shortAnswer:
      "The report records SHA-256 and SHA-512 digests of the target file's bytes. Re-hashing the same file and comparing the values shows whether the bytes changed after the scan. The hashes say nothing about who created the audio, who owns it, or when it was made; they are an integrity record, not a provenance proof.",
    technicalBasis: "SHA-256 and SHA-512 over raw file bytes; full digests in JSON",
    detail:
      "Both digests are computed over the file as stored, including its container and metadata. That is useful for evidence handling: a tag edit, a re-encode or a single flipped byte changes the hash even when the audible content is the same. It also means a matching hash identifies the exact byte sequence that was scanned, not an earlier master from which it was derived. The text certificate shortens SHA-512 for display, so compare against the JSON when the full value matters. Because both hashes are inside the HMAC-covered body, changing them breaks the seal unless the editor also holds the key.",
    trapExample:
      "A matching SHA-256 proves this file is the original master.",
    whyItFails:
      "A hash identifies bytes, not origin. Any copy with identical bytes has the same digest, and anyone can compute a hash of any file. A match only shows that the file you hashed is byte-identical to the file described in the report; it does not show who made either one or when.",
    redlineExample:
      "Compare the evidence file with the record, using both digests:\nsha256sum evidence.mp3\nsha512sum evidence.mp3\n# compare with targetFile.hashes in proof.json\nOn Windows: certutil -hashfile evidence.mp3 SHA256\nIf the values differ, stop and establish why before relying on the report.",
    limits: [
      "Integrity of the recorded bytes only.",
      "A tag or container edit changes the hash without changing the audio.",
      "No authorship, ownership, creation time or chain of custody.",
      "The text certificate shortens SHA-512; use the JSON for the full digest."
    ],
    faq: [
      {
        q: "Why record both SHA-256 and SHA-512?",
        a: "They are independent digests of the same bytes. SHA-256 is sufficient for accidental-change detection in normal use; SHA-512 adds a second, longer value at little cost. A mismatch in either is a signal to investigate."
      },
      {
        q: "Does a different hash mean the file was tampered with?",
        a: "Not necessarily. A re-encode, a metadata edit or a different export produces a different hash with no malicious intent. Compare the provenance and the audio, not just the digest."
      }
    ],
    related: ["verify-forensic-report", "hmac-seal-vs-signature", "audio-watermark-vs-metadata"]
  },
  {
    slug: "watermark-match-not-authorship",
    h1: "Does a watermark match prove authorship or ownership?",
    shortTitle: "A match is not authorship",
    category: "Forensics",
    metaDesc:
      "No. A match shows the audio carries a CRC-valid ID tied to a key and a distribution record. It does not establish who made, owns or leaked it.",
    shortAnswer:
      "A match means the detector found a CRC-valid codeword under your key. If that ID is in your manifest, it points to a distribution record. It does not establish authorship, ownership, creation time, intent or who leaked the copy; anyone with the tool and the key can embed the same ID, and an ID can be reused.",
    technicalBasis: "CRC-valid 32-bit ID + key + recipient manifest",
    detail:
      "The match is a signal-level result: the key derives a spreading sequence, the sequence carries a 32-bit payload and a CRC, and the detector found that codeword with enough confidence. The manifest then maps the ID to a record you created. Neither step asserts who created the audio, who owns the rights, or when any of it happened. The ID is a pointer, not a signed statement, and the key is shared with everyone who embeds or detects. A copy can be passed on, stolen, re-tagged by someone with the key, or averaged with other copies until recovery fails. Treat the match as a lead that connects a file to a distribution record you control.",
    trapExample:
      "A matching ID proves the recipient leaked the file.",
    whyItFails:
      "The ID identifies a copy and a record, not a person or an act. The recipient's copy can be shared or stolen, the ID can be reused, and anyone with the key can embed it in another file. The audit tool's own verdict says a match is not proof of who leaked it.",
    redlineExample:
      "Write the finding at the level the evidence supports:\n1. Report the CRC-valid ID, key and confidence.\n2. Report the manifest record (name, filename, SHA-256, date).\n3. Compare the leaked file hash with the recorded hash.\n4. Preserve the original, the copy and the detection report.\nSay \"the file carries ID X, which maps to record Y\", not \"Y leaked it\".",
    limits: [
      "A match is not authorship, ownership, creation time or chain of custody.",
      "It does not prove who leaked a copy or that a leak occurred.",
      "Anyone with the key can embed or reuse an ID.",
      "Averaging independently tagged copies can defeat recovery."
    ],
    faq: [
      {
        q: "Can I name the recipient in a report?",
        a: "You can name the record the ID maps to and the file hash you matched. You cannot conclude from the watermark alone that the recipient leaked the file; state the limit explicitly and keep the other evidence separate."
      },
      {
        q: "Can someone forge my watermark?",
        a: "Anyone who has the key and the tool can embed the same ID. The ID is not signed, and the 32-bit space is a pointer rather than an identity. Keep the key private and use contracts and independent records for attribution."
      }
    ],
    related: ["leak-audit", "hmac-seal-vs-signature", "payload-id-and-key"]
  },
  {
    slug: "ber-and-confidence",
    h1: "What do BER and confidence mean in a detection result?",
    shortTitle: "BER and confidence",
    category: "Forensics",
    metaDesc:
      "BER is the share of the 48 codeword bits that differ from the expected ID; confidence is a normalized z-score. Blind-mode BER is always 0 by design.",
    shortAnswer:
      "BER is the fraction of the 48 codeword bits that differ from the expected codeword in verify mode. Confidence is a normalized matched-filter score, not a probability. In blind mode BER is always 0 because the hard decisions are compared with themselves; use confidence, CRC validity and the z-score there.",
    technicalBasis: "48-bit codeword; BER in verify mode; confidence = clamp((z − 3) / 27); CRC-16",
    detail:
      "The detector correlates the audio with the key-derived reference codeword and forms a z-score from the aligned soft symbols. Confidence normalises that score to 0–1; it is a summary of signal evidence, not a probability that a person or owner is correct. A result counts as detected only when the CRC matches, the expected ID matches in verify mode, the confidence clears the threshold (0.5 normally, 0.35 with a passing CRC) and the recovered amplitude is positive. BER compares the hard bit decisions with the reference codeword, so it is meaningful only when an expected ID exists. The CRC-16 catches most errors but is not a cryptographic integrity check.",
    trapExample:
      "A confidence of 100% means the match is certain, and a BER of 0 proves the copy is perfect.",
    whyItFails:
      "Confidence is a capped normalisation of a correlation score, not a probability of authorship. In blind mode BER is 0 by construction, so it says nothing about quality. A CRC-valid codeword shows a consistent signal, not a perfect copy or a truthful claim about the file.",
    redlineExample:
      "Read the result in this order:\n1. crcOk and recoveredPayloadId\n2. confidence and z\n3. ber only when you supplied --id (verify mode)\n4. reps, Eb/N0 and SQNR for margin\nA valid CRC plus the expected ID is the match. Report BER as signal quality, not as proof of anything.",
    limits: [
      "Blind-mode BER is always 0; it is not a quality metric there.",
      "Confidence is a normalized z-score, not a probability.",
      "CRC-16 detects many errors but is not cryptographic.",
      "A passing threshold is a signal result, not an authorship finding."
    ],
    faq: [
      {
        q: "Why is BER 0 in blind mode?",
        a: "Without an expected ID, the hard decisions are compared against themselves, so no bits differ. Use confidence, CRC validity and z instead; BER becomes meaningful in verify mode when you supply the expected ID."
      },
      {
        q: "What confidence is good enough?",
        a: "Detection needs at least 0.5, or 0.35 when the CRC passes. A marginal pass is still a pass, but record the margin (confidence, z, reps) so a later reviewer can see how close it was."
      }
    ],
    related: ["blind-vs-verify-mode", "verify-forensic-report", "watermark-match-not-authorship"]
  },
  {
    slug: "blind-vs-verify-mode",
    h1: "Blind vs. verify mode: what is the difference?",
    shortTitle: "Blind vs. verify mode",
    category: "Forensics",
    metaDesc:
      "Blind detection recovers any CRC-valid ID under the key; verify checks an expected ID and reports a real BER. Use verify when you have the manifest.",
    shortAnswer:
      "In blind mode you supply the key but no expected ID; the detector recovers whatever CRC-valid codeword it finds. In verify mode you also supply the expected ID, so it can confirm the match and compute BER against that codeword. Blind mode is for intake; verify mode is for a known distribution record.",
    technicalBasis: "payloadId omitted = blind; supplied = verify; CRC-16 gates both",
    detail:
      "Both modes search the same bands and preamble and require a CRC-valid codeword. Blind mode reports the recovered ID and a BER of 0 by construction; verify mode additionally checks the ID, reports a real BER and can reject a CRC-valid but wrong codeword. The leak-audit script uses blind detection because it does not know which recipient to expect, then looks the recovered ID up in the manifest. If you already know the ID, verify mode is the tighter check. Neither mode works with the wrong key: a mismatched key produces noise rather than an authentication failure.",
    trapExample:
      "Blind mode is weaker because it has no expected ID, and its BER of 0 means a perfect match.",
    whyItFails:
      "Blind mode is not weaker at finding a signal; it simply has no reference ID to compare against. The BER of 0 is a definitional artifact, not evidence of quality. Use confidence, CRC and z in blind mode, and use verify mode when you have the expected ID.",
    redlineExample:
      "Blind intake on an unknown file:\nnode aureal-watermark.cjs detect leaked.mp3\n# then look up recoveredPayloadId in recipients.json\nVerify a known copy:\nnode aureal-watermark.cjs detect copy.wav --id 883921 --key \"$AUREAL_KEY\"\nRecord the mode and key with the result; a blind result and a verify result answer different questions.",
    limits: [
      "Both modes need the same key used for embedding.",
      "Blind-mode BER is always 0 and carries no quality signal.",
      "Verify mode needs the correct expected ID.",
      "A CRC-valid ID does not establish authorship or ownership."
    ],
    faq: [
      {
        q: "Which mode should I use for a leak?",
        a: "Start blind: you usually do not know which recipient's ID is present. Recover the ID, then look it up in the private manifest. Use verify mode when you have a candidate copy and want to confirm a specific ID."
      },
      {
        q: "Can blind mode find any Aureal ID?",
        a: "It can recover a CRC-valid ID embedded with the same key. It cannot see IDs embedded under a different key, and it cannot tell you who owns the file."
      }
    ],
    related: ["ber-and-confidence", "leak-audit", "payload-id-and-key"]
  },
  {
    slug: "batch-distribution",
    h1: "How do I watermark a batch of promo copies?",
    shortTitle: "Batch distribution",
    category: "Workflow",
    metaDesc:
      "Embed one ID per recipient and keep a private recipients.json with the key and SHA-256 per file. Never publish it; averaging still defeats recovery.",
    shortAnswer:
      "Use the batch script with a recipient list: it embeds a distinct ID in each copy, names the files after the master and recipient, and writes a private recipients.json mapping IDs, names, filenames and SHA-256 hashes. The manifest also records the watermark key, so treat it like the promos; averaging copies from several recipients still defeats recovery.",
    technicalBasis: "batch_distribute.js; aureal-recipients/1 manifest; per-copy SHA-256",
    detail:
      "The script reads a WAV master, embeds one payload per recipient and writes a WAV copy for each. By default it picks random IDs; --start-id pins a sequential range, which is easier to guess and ties the numbers to the order of the list. The manifest records the master metadata, band, strength, key (or null when the default was used), keyDefault, startId and a recipients array with id, name, filename and sha256. The key is in the manifest because an audit needs it to re-derive the spreading sequence. Keep the manifest offline and out of the distribution folder that recipients can browse.",
    trapExample:
      "Publish recipients.json so every recipient can check their own ID.",
    whyItFails:
      "The manifest contains the key and the full ID-to-recipient mapping. Anyone who has it can embed a matching ID in another file, and the whole distribution is exposed. Recipients do not need the manifest; they need their copy and, at most, their own ID.",
    redlineExample:
      "Tag the batch with a private key and keep the manifest offline:\nnode scripts/batch_distribute.js master.wav --recipients \"A, B\" --out-dir ./promos --key \"$AUREAL_KEY\"\nsha256sum ./promos/*.wav > promos.sha256\n# Store recipients.json and the key separately from the promos.\n# Encode delivery formats afterwards, then re-test detection.",
    limits: [
      "The manifest is private: it contains the key and the mapping.",
      "Averaging copies from several recipients defeats recovery.",
      "The batch script reads and writes WAV; re-test after encoding.",
      "An ID maps to a recipient record, not to a person or a proven leak."
    ],
    faq: [
      {
        q: "Random or sequential IDs?",
        a: "Random is the default because sequential IDs are guessable and follow the order of the recipient list. Use --start-id only when you need a pinned range, and still keep the manifest private."
      },
      {
        q: "Can every recipient get the same ID?",
        a: "You can, but then a match cannot distinguish recipients. The point of per-recipient IDs is to narrow a leaked copy to one distribution record."
      }
    ],
    related: ["leak-audit", "key-management", "watermark-match-not-authorship"]
  },
  {
    slug: "leak-audit",
    h1: "How do I trace a leaked promo copy?",
    shortTitle: "Leak audit",
    category: "Workflow",
    metaDesc:
      "Run blind detection on the leaked file, then look up the ID in your private manifest. A hit names a distribution record, not the person who leaked it.",
    shortAnswer:
      "Run the leak audit with the leaked file and your private recipients.json. It normalises the input to WAV when needed, runs blind detection with the manifest key, and looks up the recovered ID. A hit names the recipient record and shows the original filename and SHA-256; the verdict is a lead, not proof of who leaked the copy.",
    technicalBasis: "audit_leak.js; blind detection + manifest ID lookup; ffmpeg when needed",
    detail:
      "Non-WAV input is decoded with ffmpeg to 44.1 kHz WAV before detection; a missing ffmpeg is an explicit failure, not a silent skip. The script takes the key from the manifest unless --key overrides it, runs blind detection, maps recoveredPayloadId to the recipients array and prints the record. The human report includes confidence, BER, CRC status, filename, original SHA-256 and distribution date; --json returns the detection object and a leakMatch. Exit status is 0 when a watermark was detected, 1 when none was. Preserve the leaked file and its hash before running anything.",
    trapExample:
      "The audit names the leaker, so the report is ready to send as a complaint.",
    whyItFails:
      "The report names the record whose ID was recovered. It cannot show who copied, shared or leaked the file, whether the ID was reused, or how the file was handled. Use the result as one lead among contracts, delivery records, hashes and other evidence; state the limitation in the report.",
    redlineExample:
      "Hash the evidence, then audit it:\nsha256sum leaked.mp3\nnode scripts/audit_leak.js leaked.mp3 --manifest ./private/recipients.json --json\n# exit 0 = watermark detected; 1 = none\n# Record the tool version, key reference, manifest hash and the full output.\n# Describe the hit as an ID-to-record match, not as proof of leaking.",
    limits: [
      "A match names a distribution record, not a person or an act.",
      "The manifest must be private, intact and the same one used for distribution.",
      "Short clips, codecs and wrong keys can produce a false negative.",
      "The audit does not establish chain of custody."
    ],
    faq: [
      {
        q: "What if no watermark is found?",
        a: "That is not proof of innocence. The mark may have been removed by processing, the clip may be shorter than a frame, the wrong key or manifest may be in use, or the file may never have carried a mark. Record the negative result and its conditions."
      },
      {
        q: "Can I audit without the manifest?",
        a: "You can run blind detection and recover an ID, but without the manifest you cannot map it to a recipient record or a filename. The manifest is the private lookup table; detection alone is not attribution."
      }
    ],
    related: ["batch-distribution", "watermark-match-not-authorship", "blind-vs-verify-mode"]
  },
  {
    slug: "key-management",
    h1: "How should I manage the watermark key?",
    shortTitle: "Key management",
    category: "Workflow",
    metaDesc:
      "The key is an obfuscation-grade PRNG seed, not an encryption key. Lose it and detection fails; publish it and anyone can reproduce your IDs.",
    shortAnswer:
      "The key seeds the spreading sequence used to embed and detect the ID. It is obfuscation-grade, not a cryptographic passkey: a wrong key produces noise rather than an authentication error, and it does not encrypt the audio. Keep one strong secret per campaign, store it with the private manifest, and never ship it with the promos.",
    technicalBasis: "FNV-1a seeded PN sequences; shared secret; fixed public default",
    detail:
      "The key is hashed to a 32-bit seed that drives the pseudorandom sequence; the same key must be used at embed and detect time. There is no cryptographic key derivation, rotation or revocation in the file format, so changing the key means re-embedding and keeping the old key for old copies. The built-in default is a fixed public constant; a file marked with it can be detected, and matched, by anyone. The batch manifest stores the key because an audit needs it, which makes the manifest as sensitive as the promos. Re-marking a file with the same key and a conflicting ID can destroy the original ID.",
    trapExample:
      "The key encrypts the audio, so a leaked promo is unreadable without it.",
    whyItFails:
      "Watermarking does not encrypt or restrict playback; the audio is fully listenable. The key only controls the spreading sequence and the detection result. A wrong key yields noise, not a decryption failure, and the default key is public.",
    redlineExample:
      "Use a unique, high-entropy key per campaign and separate it from the audio:\n1. Generate the key in a secret manager, not in a shell history.\n2. Pass it to the batch tool from the environment.\n3. Store it with the private manifest, not in the promo folder.\n4. Back it up; losing it means the old copies cannot be detected.\nNever commit the key or include it in a public report.",
    limits: [
      "Obfuscation-grade seed, not encryption or authentication.",
      "The built-in default key is public; use it only for tests.",
      "No rotation or revocation; changing the key requires re-embedding.",
      "The manifest contains the key and must stay private."
    ],
    faq: [
      {
        q: "Can I change the key later?",
        a: "New copies can use a new key, but old copies still need the old key to detect. There is no in-file rotation or revocation, so keep the old key for as long as the old distribution matters, and re-embed if you need one key across everything."
      },
      {
        q: "What happens if I use the default key?",
        a: "Anyone can detect files marked with it, and anyone can embed a matching ID. It is a test default, not a distribution secret. Use the --key option for real work."
      }
    ],
    related: ["batch-distribution", "leak-audit", "payload-id-and-key"]
  },
  {
    slug: "eu-ai-act-article-50",
    h1: "Does audio watermarking satisfy EU AI Act Article 50?",
    shortTitle: "EU AI Act Article 50",
    category: "Compliance",
    metaDesc:
      "Article 50 requires providers to mark synthetic audio machine-readably from 2 August 2026. A watermark is one measure; it is not a compliance certificate.",
    shortAnswer:
      "Article 50 of the EU AI Act requires providers of generative AI systems to mark synthetic audio in a machine-readable, detectable way, and deployers to disclose deepfakes. It applies from 2 August 2026. A watermark can be part of that, but Aureal does not certify compliance, and a mark that can be removed or missed is not sufficient on its own.",
    technicalBasis: "Regulation (EU) 2024/1689, Articles 50 and 111(4); Commission Article 50 FAQ (2026)",
    detail:
      "Article 50(2) puts the marking duty on providers of AI systems that generate synthetic audio, image, video or text: outputs must be marked in a machine-readable format and be detectable as artificially generated or manipulated, taking account of the specificities and limitations of the content, cost and the state of the art. Article 50(4) puts a separate disclosure duty on deployers of deepfakes, defined in Article 3(60); the Commission's guidance says a deployer cannot rely solely on the machine-readable mark to meet it. The duties apply from 2 August 2026, with a transition under Article 111(4) to 2 December 2026 for systems already placed on the market; confirm the current guidance for your case. A watermark is one technical measure, not a certification, and compliance depends on the role, the system, the output and the rest of the controls.",
    trapExample:
      "Adding an Aureal watermark makes the product Article 50 compliant.",
    whyItFails:
      "Article 50 places duties on providers and deployers, not on a file format. The marking must be effective, reliable, robust and interoperable, and deployers still owe a separate disclosure. A watermark that can be stripped, missed on a short clip, or lost at 8 kHz is one measure among several; Aureal issues no compliance determination.",
    redlineExample:
      "Map the obligation before choosing the control:\n1. Identify whether you are a provider, a deployer or both.\n2. Identify the synthetic output and where it is placed on the EU market.\n3. Document the marking method and test it across the delivery chain.\n4. Keep the deployer disclosure separate from the machine-readable mark.\n5. Get qualified legal advice; do not print \"Article 50 compliant\" without it.",
    limits: [
      "Context only; Aureal does not certify Article 50 compliance.",
      "A watermark is not sufficient on its own.",
      "Short clips, codecs and 8 kHz paths can lose the mark.",
      "Duties depend on your role, system and jurisdiction; get legal advice."
    ],
    faq: [
      {
        q: "Is a watermark enough for Article 50?",
        a: "It can be part of a marking measure, but the obligation is on the provider or deployer and requires more than a single technique. The mark must be effective, reliable, robust and interoperable for the system, and deployers have a separate disclosure duty."
      },
      {
        q: "Does Aureal certify compliance?",
        a: "No. Aureal is a watermarking tool, not a conformity assessment or a legal opinion. It can produce technical evidence; the compliance determination belongs to your counsel and, where applicable, the market-surveillance framework."
      }
    ],
    related: ["c2pa-and-audio-watermarking", "deepfake-detection", "ai-voice-providers"]
  },
  {
    slug: "c2pa-and-audio-watermarking",
    h1: "Is C2PA the same as audio watermarking?",
    shortTitle: "C2PA and watermarking",
    category: "Compliance",
    metaDesc:
      "C2PA Content Credentials are signed provenance manifests attached to a file; a watermark lives in the samples. They differ, and can be combined.",
    shortAnswer:
      "C2PA Content Credentials are cryptographically signed provenance records attached to an asset. An audio watermark is embedded in the samples. C2PA can be stripped with metadata; a watermark can survive some processing but carries only a small ID. Aureal does not create, validate or recover C2PA manifests, and the repository's bridge example is illustrative.",
    technicalBasis: "C2PA manifest (metadata/provenance) vs. sample-domain watermark",
    detail:
      "A C2PA manifest records who signed an asset, what actions were taken and the bindings to the bytes; it travels as metadata or a sidecar and can be removed or invalidated by re-encoding. A watermark is written into the audio and can survive some transcodes, but it carries a 32-bit ID, not a manifest. The two layers answer different questions: C2PA is a signed statement at the point of signing; a watermark is a signal after the statement is gone. They can be combined, with the ID as an application-level lookup to a record you keep, but combining them does not make either one a compliance certificate.",
    trapExample:
      "Aureal recovers the C2PA manifest after a platform strips the metadata.",
    whyItFails:
      "Aureal recovers its own watermark ID. It does not store, validate or reconstruct a C2PA manifest, and the repository's bridge example is illustrative rather than a certified implementation. A recovered ID can point to a record, not to a signed C2PA claim.",
    redlineExample:
      "Keep the two layers separate in the record:\n1. Sign and attach the C2PA manifest at export.\n2. Embed the watermark ID in the audio.\n3. Store the ID-to-record mapping privately.\n4. If metadata is stripped, look up the ID in your record; do not claim the manifest survived.\nTest both layers on the actual platform.",
    limits: [
      "Aureal is not a C2PA implementation.",
      "A watermark carries a 32-bit ID, not a provenance manifest.",
      "Both layers can be removed or degraded by processing.",
      "No regulatory or standards certification is included."
    ],
    faq: [
      {
        q: "Does Aureal support C2PA?",
        a: "No. The repository contains an illustrative bridge example, but Aureal does not create, validate or recover C2PA manifests. The ID can be used alongside an independently maintained provenance record."
      },
      {
        q: "Which layer survives social platforms?",
        a: "Neither is guaranteed. Metadata is routinely stripped, and a watermark can be lost to short clips, heavy processing or narrowband delivery. Test the actual platform and keep the two layers independent."
      }
    ],
    related: ["audio-watermark-vs-metadata", "eu-ai-act-article-50", "watermark-match-not-authorship"]
  },
  {
    slug: "deepfake-detection",
    h1: "Can Aureal detect AI-generated or deepfake audio?",
    shortTitle: "AI audio and deepfakes",
    category: "Compliance",
    metaDesc:
      "No. Aureal detects its own embedded watermark, not synthetic speech. A missing mark does not mean human; a present mark does not mean AI.",
    shortAnswer:
      "No. Aureal is not a synthetic-speech or deepfake classifier. It detects a keyed watermark that someone embedded in the audio, so it can trace marked copies. A missing watermark does not show the audio is human, and a present one does not show it is AI-generated; it only shows the mark is there.",
    technicalBasis: "Known-signal watermark detection, not generative-content classification",
    detail:
      "A synthetic-speech detector is a classifier: it generalises from learned features to an estimate about how the audio was made. Aureal does the opposite. It correlates the samples with a key-derived sequence and reports whether its own codeword is present. If an AI voice provider embedded an Aureal ID, the detector can recover that ID and trace the generation record. If the provider used no mark, a different watermark, or the mark was removed by re-recording, denoising, a short clip or a codec, the detector finds nothing. Absence is not evidence that the audio is human, and presence is not evidence that it is synthetic.",
    trapExample:
      "Aureal can tell whether a voice is AI-generated by scanning for its watermark.",
    whyItFails:
      "The detector only knows its own keyed codeword. It has no model of synthetic speech and cannot classify unmarked audio. A marked human recording would match; an unmarked synthetic recording would not. The result describes the presence of a mark, not the origin of the voice.",
    redlineExample:
      "Write the result at signal level:\n\"The file carries watermark ID 883921 under the supplied key\"\nor\n\"No watermark was detected under the supplied key\".\nDo not write \"AI-generated\", \"human\" or \"deepfake\" from a watermark result.\nIf origin matters, use a provenance record, a classifier evaluated for that task, and the applicable disclosure rules.",
    limits: [
      "No synthetic-speech or deepfake classification.",
      "Only detects an Aureal-compatible mark under the matching key.",
      "A missing mark does not prove the audio is human.",
      "A present mark does not prove the audio is AI-generated."
    ],
    faq: [
      {
        q: "Can it detect audio from a specific AI voice vendor?",
        a: "Only if that vendor embedded an Aureal-compatible watermark with the key you hold, or the file carries one from another step in the chain. Aureal has no vendor-specific classifier and cannot identify a model from an unmarked file."
      },
      {
        q: "Does a missing watermark prove the audio is real?",
        a: "No. The mark may never have been embedded, may have been removed by processing, or the clip may be too short. Absence is not evidence of human origin; it is only the absence of this signal under this key."
      }
    ],
    related: ["ai-voice-providers", "eu-ai-act-article-50", "watermark-match-not-authorship"]
  },
  {
    slug: "ai-voice-providers",
    h1: "Can AI voice providers watermark synthetic speech?",
    shortTitle: "AI voice providers",
    category: "Use case",
    metaDesc:
      "Yes, as an integration: embed a per-generation ID after synthesis, keep the mapping private, and test the vocoder, codec and 8 kHz paths.",
    shortAnswer:
      "A provider of synthetic speech can embed a distinct ID in each generated file, keep a private mapping from ID to generation record, and use detection to trace copies. That is an integration pattern, not a compliance certification. Vocoders, denoisers, codecs and narrowband paths can remove or weaken the mark, so test the actual output chain.",
    technicalBasis: "Post-synthesis embedding; per-generation ID; private manifest",
    detail:
      "Embed after the final audio stage, not inside the model: the watermark is applied to samples, so resynthesis, denoising or a later vocoder pass can remove it. Most speech output is 24 or 48 kHz, where the mid band is the practical choice; the high band may be filtered by the vocoder or the player. Telephony and voice-assistant paths at 8 kHz are a documented weak case. Very short outputs (under about one second) cannot carry a complete frame. Keep one ID per generation or per customer, store the mapping with the key in a private system, and re-test after every change to the rendering or delivery chain.",
    trapExample:
      "Embed once in the model, and every output is automatically marked and compliant.",
    whyItFails:
      "The watermark is applied to audio, not to model weights, and later processing can remove it. Short outputs, 8 kHz delivery and aggressive denoising are known weak paths. Marking is also only one part of a transparency obligation; it is not a compliance certificate.",
    redlineExample:
      "Integrate at the audio boundary and test the real chain:\n1. Render the final speech, including the vocoder.\n2. Embed a per-generation ID with a private campaign key.\n3. Store ID, generation record, key reference and SHA-256 privately.\n4. Test the delivered format, including the 8 kHz path if you offer one.\n5. Re-test after any change to the renderer, codec or platform.",
    limits: [
      "Post-synthesis integration, not model-level marking.",
      "Short outputs, 8 kHz and heavy denoising are weak paths.",
      "No Article 50 certification or legal determination.",
      "Per-generation key and manifest management are the provider's responsibility."
    ],
    faq: [
      {
        q: "Where in the pipeline should I embed?",
        a: "After synthesis and the final audio processing, before distribution. If you re-render, denoise or transcode afterwards, test the result; a mark added before a vocoder may not survive it."
      },
      {
        q: "Does this satisfy Article 50?",
        a: "It can be one technical measure. Article 50 puts duties on providers and deployers, requires effective and robust marking, and adds a separate disclosure duty for deepfakes. Aureal does not certify compliance; assess the whole system with counsel."
      }
    ],
    related: ["deepfake-detection", "eu-ai-act-article-50", "batch-distribution"]
  }
];
