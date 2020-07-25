# bm

![Crates.io](https://img.shields.io/crates/v/bm.svg)
![Docs](https://docs.rs/bm/badge.svg)

Binary merkle tree implementation with support of merkleization,
de-merkliezation and in-place tree modification.

* SimpleSerialize (ssz) compliant.
* Garbage collection support.

## Concepts

We distingish `Intermediate` values and `End` values so that
implementations can better handle preimage attacks.

* `Intermediate`: a node that has two direct children `left` and
  `right`.
* `End`: a node without any child.

## Backend

The library `bm` provides two basic backends:

* `InMemoryBackend`: a backend that stores all merkle nodes in-memory.
* `NoopBackend`: dummy backend that discards any `set` operation, and
  return error on any `get` operation. This is useful if you're
  interested in the merkle root but does not actually need the merkle
  tree.

## In-place Tree Modification

You can use `bm` library for in-place tree modification. To use the
ssz compliant variant, choose `new_with_inherited_empty` when creating
the backend and set `List`, `PackedList`'s maximum length to an
approriate value.

* `Raw`: Raw binary merkle tree that allows directly operating on
  generalized merkle index.
* `List`: Variable-sized vector list.
* `Vector`: Fixed-sized tuple list.
* `PackedList`: Packed variable-sized vector list.
* `PackedVector`: Packed fixed-sized tuple list.

## Merkleization

You can use `bm-le` library for merkleization. It is ssz compatibile
and with some extensions to make it work better in certain
environments. If you're only interested in the merkle root, use
`tree_root` function. Otherwise, use `IntoTree` trait.

In order to merkleize vectors and lists, use `FixedVec` and
`VariableVec` wrapper type. To merkleize bitvectors and bitlists, use
`FixedVec<bool>` and `VariableVec<bool>`.

## Demerkleization

Because some information are not available on type (like vector's
length, and vector and list's maximum length), we use three traits for
demerkleization support -- `FromTree`, `FromListTree` and
`FromVectorTree`.

## Basic Usage

See `tests/ssz.rs` for basic usage examples.
