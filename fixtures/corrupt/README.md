<<<<<<< HEAD
# corrupt/

Truncated/invalid Matroska/WebM/Ogg headers. Indexing must record these as
corrupt without crashing (§17, §25).
=======
# corrupt

Malformed/truncated media fixtures: indexing must classify and continue — never crash (spec §17, §19.2, §25: "a corrupt or malformed media file cannot crash an indexing run").

**Status: to be generated in Phase 1.** Each file will document its corruption shape (truncated container, bad codec payload, zero-byte file, wrong extension) and the expected archive-health finding (§12).
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
