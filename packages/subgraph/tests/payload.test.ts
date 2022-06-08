import { clearStore, test, assert } from "matchstick-as/assembly/index";
import { processPayload } from "../src/mapping";
import { Bytes } from "@graphprotocol/graph-ts";

test("Payload empty blocknums", () => {
  let payloadBytes = Bytes.fromHexString("0x00") as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  // To Do add asserts

  clearStore();
});

test("Payload processing latest example", () => {
  let payloadBytes = Bytes.fromHexString(
    "0x0c2901090d413a313939310b423a326b6c0b433a3139300f443a31383831386c5fd2e9c3875bbbc8533fb99bfeefe7da0877ec424bf19d1b2831a9f84cf476016209c212221c2cd36745f8ecf16243c11ca6bbd507dea7a452beea7b0ac31093dabafdb9f1356a09055609b612006848a26c8bded1673259a391cb548013c6ea0640f2ff38f390917c15f0da9b8801010101c76430fea08e4b1ee3e427d55cc814386bb1ac0d63536e35928b120f5d4f7bd701010101"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  // To Do add asserts

  clearStore();
});
