use sha2::{Digest, Sha256};
use primitive_types::H256;
use std::fmt::Debug;
use std::str::FromStr;
use typenum::*;

use bm::InMemoryBackend;
use generic_array::GenericArray;
use bm_le::{IntoTree, FromTree, Compact, MaxVec, DigestConstruct};

fn chunk(data: &[u8]) -> H256 {
	let mut ret = [0; 32];
	ret[..data.len()].copy_from_slice(data);

	H256::from(ret)
}

fn h(a: &[u8], b: &[u8]) -> H256 {
	let mut hash = Sha256::new();
	hash.input(a);
	hash.input(b);
	H256::from_slice(hash.result().as_slice())
}

fn t<T>(value: T, expected: H256) where
	T: IntoTree + FromTree,
	T: Debug + PartialEq,
{
	let mut db = InMemoryBackend::<DigestConstruct<Sha256>>::default();
	let actual = value.into_tree(&mut db).unwrap();
	assert_eq!(H256::from_slice(actual.as_ref()), expected);
	let decoded = T::from_tree(&actual, &mut db).unwrap();
	assert_eq!(value, decoded);
}

#[test]
fn spec() {
	t(false, chunk(&[0x00])); // boolean F
	t(true, chunk(&[0x01])); // boolean T
	t(0u8, chunk(&[0x00])); // uint8 00
	t(1u8, chunk(&[0x01])); // uint8 01
	t(0xabu8, chunk(&[0xab])); // uint8 ab
	t(0x0000u16, chunk(&[0x00, 0x00])); // uint16 0000
	t(0xabcdu16, chunk(&[0xcd, 0xab])); // uint16 abcd
	t(0x00000000u32, chunk(&[0x00, 0x00, 0x00, 0x00])); // uint32 00000000
	t(0x01234567u32, chunk(&[0x67, 0x45, 0x23, 0x01])); // uint32 01234567

	// uint64 0000000000000000
	t(0x0000000000000000u64, chunk(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
	// uint64 0123456789abcdef
	t(0x0123456789abcdefu64, chunk(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]));

	// bitvector TTFTFTFF
	t(Compact(GenericArray::<bool, U8>::from([true, true, false, true, false, true, false, false])), chunk(&[0x2b]));
	// bitvector FTFT
	t(Compact(GenericArray::<bool, U4>::from([false, true, false, true])), chunk(&[0x0a]));
	// bitvector FTF
	t(Compact(GenericArray::<bool, U3>::from([false, true, false])), chunk(&[0x02]));
	// bitvector TFTFFFTTFT
	t(Compact(GenericArray::<bool, U10>::from([true, false, true, false, false, false, true, true, false, true])),
			chunk(&[0xc5, 0x02]));
	// bitvector TFTFFFTTFTFFFFTT
	t(Compact(GenericArray::<bool, U16>::from([
		true, false, true, false, false, false, true, true, false, true,
		false, false, false, false, true, true
	])), chunk(&[0xc5, 0xc2]));
	// long bitvector
	{
		let mut v = Vec::new();
		for _ in 0..512 {
			v.push(true);
		}
		t(
			Compact(GenericArray::<bool, U512>::from_exact_iter(v).unwrap()),
			h(&[
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
			], &[
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
				0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
			])
		);
	}

	// bitlist TTFTFTFF
	t(
		Compact(MaxVec::<bool, U8>::from(vec![true, true, false, true, false, true, false, false])),
		h(&chunk(&[0x2b])[..], &chunk(&[0x08])[..])
	);
	// bitlist FTFT
	t(
		Compact(MaxVec::<bool, U4>::from(vec![false, true, false, true])),
		h(&chunk(&[0x0a])[..], &chunk(&[0x04])[..])
	);
	// bitlist FTF
	t(
		Compact(MaxVec::<bool, U3>::from(vec![false, true, false])),
		h(&chunk(&[0x02])[..], &chunk(&[0x03])[..])
	);
	// bitlist TFTFFFTTFT
	t(
		Compact(MaxVec::<bool, U16>::from(vec![true, false, true, false, false, false, true, true, false, true])),
		h(&chunk(&[0xc5, 0x02])[..], &chunk(&[0x0a])[..])
	);
	// bitlist TFTFFFTTFTFFFFTT
	t(
		Compact(MaxVec::<bool, U16>::from(vec![
			true, false, true, false, false, false, true, true, false, true,
			false, false, false, false, true, true])
		), h(&chunk(&[0xc5, 0xc2])[..], &chunk(&[0x10])[..])
	);
	t(
		Compact(MaxVec::<bool, U4096>::from(vec![
			true, false, true, true, true, false, false, false
		])),
		H256::from_str("f4de82badf841b3e8064de143959343ec7d4405e72d95bfc741748bb15721ff4").unwrap()
	);
	t(GenericArray::<H256, U0>::from_exact_iter(vec![]).unwrap(), H256::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap());
}

// test_data = [
//	   ("long bitlist", Bitlist[512](1),
//		"03", h(h(chunk("01"), chunk("")), chunk("01"))),
//	   ("long bitlist", Bitlist[512](1 for i in range(512)),
//		"ff" * 64 + "01", h(h("ff" * 32, "ff" * 32), chunk("0002"))),
//	   ("odd bitvector", Bitvector[513](1 for i in range(513)),
//		"ff" * 64 + "01", h(h("ff" * 32, "ff" * 32), h(chunk("01"), chunk("")))),
//	   ("odd bitlist", Bitlist[513](1 for i in range(513)),
//		"ff" * 64 + "03", h(h(h("ff" * 32, "ff" * 32), h(chunk("01"), chunk(""))), chunk("0102"))),
//	   ("small (4567, 0123)", SmallTestStruct(A=0x4567, B=0x0123), "67452301", h(chunk("6745"), chunk("2301"))),
//	   ("small [4567, 0123]::2", Vector[uint16, 2](uint16(0x4567), uint16(0x0123)), "67452301", chunk("67452301")),
//	   ("sig", BytesN[96](*sig_test_data),
//		"0100000000000000000000000000000000000000000000000000000000000000"
//		"0200000000000000000000000000000000000000000000000000000000000000"
//		"03000000000000000000000000000000000000000000000000000000000000ff",
//		h(h(chunk("01"), chunk("02")),
//		  h("03000000000000000000000000000000000000000000000000000000000000ff", chunk("")))),
//	   ("emptyTestStruct", EmptyTestStruct(), "", chunk("")),
//	   ("singleFieldTestStruct", SingleFieldTestStruct(A=0xab), "ab", chunk("ab")),
//	   ("uint16 list", List[uint16, 32](uint16(0xaabb), uint16(0xc0ad), uint16(0xeeff)), "bbaaadc0ffee",
//		h(h(chunk("bbaaadc0ffee"), chunk("")), chunk("03000000"))  # max length: 32 * 2 = 64 bytes = 2 chunks
//		),
//	   ("uint32 list", List[uint32, 128](uint32(0xaabb), uint32(0xc0ad), uint32(0xeeff)), "bbaa0000adc00000ffee0000",
//		# max length: 128 * 4 = 512 bytes = 16 chunks
//		h(merge(chunk("bbaa0000adc00000ffee0000"), zero_hashes[0:4]), chunk("03000000"))
//		),
//	   ("uint256 list", List[uint256, 32](uint256(0xaabb), uint256(0xc0ad), uint256(0xeeff)),
//		"bbaa000000000000000000000000000000000000000000000000000000000000"
//		"adc0000000000000000000000000000000000000000000000000000000000000"
//		"ffee000000000000000000000000000000000000000000000000000000000000",
//		h(merge(h(h(chunk("bbaa"), chunk("adc0")), h(chunk("ffee"), chunk(""))), zero_hashes[2:5]), chunk("03000000"))
//		),
//	   ("uint256 list long", List[uint256, 128](i for i in range(1, 20)),
//		"".join([i.to_bytes(length=32, byteorder='little').hex() for i in range(1, 20)]),
//		h(merge(
//			h(
//				h(
//					h(
//						h(h(chunk("01"), chunk("02")), h(chunk("03"), chunk("04"))),
//						h(h(chunk("05"), chunk("06")), h(chunk("07"), chunk("08"))),
//					),
//					h(
//						h(h(chunk("09"), chunk("0a")), h(chunk("0b"), chunk("0c"))),
//						h(h(chunk("0d"), chunk("0e")), h(chunk("0f"), chunk("10"))),
//					)
//				),
//				h(
//					h(
//						h(h(chunk("11"), chunk("12")), h(chunk("13"), chunk(""))),
//						zero_hashes[2]
//					),
//					zero_hashes[3]
//				)
//			),
//			zero_hashes[5:7]), chunk("13000000"))  # 128 chunks = 7 deep
//		),
//	   ("fixedTestStruct", FixedTestStruct(A=0xab, B=0xaabbccdd00112233, C=0x12345678), "ab33221100ddccbbaa78563412",
//		h(h(chunk("ab"), chunk("33221100ddccbbaa")), h(chunk("78563412"), chunk("")))),
//	   ("varTestStruct nil", VarTestStruct(A=0xabcd, C=0xff), "cdab07000000ff",
//		h(h(chunk("cdab"), h(zero_hashes[6], chunk("00000000"))), h(chunk("ff"), chunk("")))),
//	   ("varTestStruct empty", VarTestStruct(A=0xabcd, B=List[uint16, 1024](), C=0xff), "cdab07000000ff",
//		h(h(chunk("cdab"), h(zero_hashes[6], chunk("00000000"))), h(chunk("ff"), chunk("")))),	# log2(1024*2/32)= 6 deep
//	   ("varTestStruct some", VarTestStruct(A=0xabcd, B=List[uint16, 1024](1, 2, 3), C=0xff),
//		"cdab07000000ff010002000300",
//		h(
//			h(
//				chunk("cdab"),
//				h(
//					merge(
//						chunk("010002000300"),
//						zero_hashes[0:6]
//					),
//					chunk("03000000")  # length mix in
//				)
//			),
//			h(chunk("ff"), chunk(""))
//	   )),
//	   ("complexTestStruct",
//		ComplexTestStruct(
//			A=0xaabb,
//			B=List[uint16, 128](0x1122, 0x3344),
//			C=0xff,
//			D=Bytes[256](b"foobar"),
//			E=VarTestStruct(A=0xabcd, B=List[uint16, 1024](1, 2, 3), C=0xff),
//			F=Vector[FixedTestStruct, 4](
//				FixedTestStruct(A=0xcc, B=0x4242424242424242, C=0x13371337),
//				FixedTestStruct(A=0xdd, B=0x3333333333333333, C=0xabcdabcd),
//				FixedTestStruct(A=0xee, B=0x4444444444444444, C=0x00112233),
//				FixedTestStruct(A=0xff, B=0x5555555555555555, C=0x44556677)),
//			G=Vector[VarTestStruct, 2](
//				VarTestStruct(A=0xdead, B=List[uint16, 1024](1, 2, 3), C=0x11),
//				VarTestStruct(A=0xbeef, B=List[uint16, 1024](4, 5, 6), C=0x22)),
//		),
//		"bbaa"
//		"47000000"	# offset of B, []uint16
//		"ff"
//		"4b000000"	# offset of foobar
//		"51000000"	# offset of E
//		"cc424242424242424237133713"
//		"dd3333333333333333cdabcdab"
//		"ee444444444444444433221100"
//		"ff555555555555555577665544"
//		"5e000000"	# pointer to G
//		"22114433"	# contents of B
//		"666f6f626172"	# foobar
//		"cdab07000000ff010002000300"  # contents of E
//		"08000000" "15000000"  # [start G]: local offsets of [2]varTestStruct
//		"adde0700000011010002000300"
//		"efbe0700000022040005000600",
//		h(
//			h(
//				h(	# A and B
//					chunk("bbaa"),
//					h(merge(chunk("22114433"), zero_hashes[0:3]), chunk("02000000"))  # 2*128/32 = 8 chunks
//				),
//				h(	# C and D
//					chunk("ff"),
//					h(merge(chunk("666f6f626172"), zero_hashes[0:3]), chunk("06000000"))  # 256/32 = 8 chunks
//				)
//			),
//			h(
//				h(	# E and F
//					h(h(chunk("cdab"), h(merge(chunk("010002000300"), zero_hashes[0:6]), chunk("03000000"))),
//					  h(chunk("ff"), chunk(""))),
//					h(
//						h(
//							h(h(chunk("cc"), chunk("4242424242424242")), h(chunk("37133713"), chunk(""))),
//							h(h(chunk("dd"), chunk("3333333333333333")), h(chunk("cdabcdab"), chunk(""))),
//						),
//						h(
//							h(h(chunk("ee"), chunk("4444444444444444")), h(chunk("33221100"), chunk(""))),
//							h(h(chunk("ff"), chunk("5555555555555555")), h(chunk("77665544"), chunk(""))),
//						),
//					)
//				),
//				h(	# G and padding
//					h(
//						h(h(chunk("adde"), h(merge(chunk("010002000300"), zero_hashes[0:6]), chunk("03000000"))),
//						  h(chunk("11"), chunk(""))),
//						h(h(chunk("efbe"), h(merge(chunk("040005000600"), zero_hashes[0:6]), chunk("03000000"))),
//						  h(chunk("22"), chunk(""))),
//					),
//					chunk("")
//				)
//			)
//		))
// ]
