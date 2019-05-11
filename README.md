# Experiments on Predecessor Search

This is a repository for two separate final projects that I completed:
one for an advanced databases seminar and one for an advanced data
structures seminar.

This has implementations of several different data structures:
- [X-fast](https://en.wikipedia.org/wiki/X-fast_trie) and
  [Y-fast](https://en.wikipedia.org/wiki/Y-fast_trie) tries, which are
  ordered map specialized for fixed-sized byte strings (and implicitly
  word-sized integers). Instead of a normal trie, which executes in
  O(l) time for keys of length l, these tries do a binary search over
  the key length to do lookups + predecessor/successor queries in O(lg
  n) time.
  - Strictly speaking, X-fast and Y-fast tries are bitwise tries. This
    implementation uses byte-wise tries instead, making the obvious
    modifications to descendant pointers to keep things working.
  - For some reason, Y-fast tries are *really* slow on my benchmarks,
    much slower than hash maps or B-trees. I don't know why -- `perf`
    indicates that we are getting a ton of cache misses, but I don't
    know *why* Y-fast tries have more cache references in the first
    place! For 10,000,000 32-bit keys, it looks searching the
    "last-level" B-trees in the Y-fast tries (which are guaranteed to
    have <64 elemens in them) causes an ungodly number of cache misses,
    way more than I'd expect.
- Byte-map, which is an ordered-map specialized for byte keys. It's
  pretty directly inspired by the [Adaptive Radix
  Trie](https://db.in.tum.de/~leis/papers/ART.pdf), although I've
  skipped most of the SIMD optimizations. It's not actually particularly
  well-optimized, but still faster than a B-tree.
- Succinct LOUDS tries -- this uses the Level-Order Unary Degree
  Sequence representation to store the tree structure of a byte trie in
  only a few bits. It comes with fast implementations for [rank/select
  bit-vectors](https://en.wikipedia.org/wiki/Succinct_data_structure#Succinct_dictionaries),
  where we trade off space for hardware efficiency.
  - Note that our bit vectors are not actually succinct, but we make good use of
  Intel's BMI instructions, [horizontal bit
  parallelism](http://pages.cs.wisc.edu/~jignesh/publ/BitWeaving.pdf)
  and SIMD instructions to get what I believe is state-of-the-art
  performance.
  - While our LOUDS tries successfully save a lot of space compared to a
    B-tree, their lookup and insertion performance is pretty miserable.
