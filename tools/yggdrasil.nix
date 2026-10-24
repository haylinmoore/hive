{ lib, ... }:

let
  hexDigits = lib.stringToCharacters "0123456789abcdef";

  nibble = c: lib.lists.findFirstIndex (d: d == c) (throw "yggdrasil: bad hex digit ${c}") hexDigits;

  bitsOf =
    hex:
    lib.concatMap (
      c:
      let
        n = nibble c;
      in
      [
        (n / 8)
        (lib.mod (n / 4) 2)
        (lib.mod (n / 2) 2)
        (lib.mod n 2)
      ]
    ) (lib.stringToCharacters (lib.toLower hex));

  byteBits =
    n:
    map (p: lib.mod (n / p) 2) [
      128
      64
      32
      16
      8
      4
      2
      1
    ];

  chunks = n: l: lib.genList (i: lib.sublist (i * n) n l) (lib.length l / n);

  hexOf = bits: lib.elemAt hexDigits (lib.foldl (a: b: a * 2 + b) 0 bits);

  trimZeros =
    s:
    let
      t = lib.removePrefix "0" s;
    in
    if s == "0" then
      s
    else if t == s then
      s
    else
      trimZeros t;

  groups = bits: map (g: trimZeros (lib.concatMapStrings hexOf (chunks 4 g))) (chunks 16 bits);

  derive =
    prefix: len: publicKey:
    assert lib.assertMsg (
      lib.stringLength publicKey == 64
    ) "yggdrasil: public key must be 64 hex characters";
    let
      inverted = map (b: 1 - b) (bitsOf publicKey);
      ones = lib.lists.findFirstIndex (b: b == 0) 256 inverted;
      rest = lib.drop (ones + 1) inverted ++ lib.replicate 128 0;
    in
    byteBits prefix ++ byteBits ones ++ lib.take (len - 16) rest;
in
{
  address = publicKey: lib.concatStringsSep ":" (groups (derive 2 128 publicKey));

  subnet = publicKey: lib.concatStringsSep ":" (groups (derive 3 64 publicKey)) + "::/64";
}
