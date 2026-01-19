# 1.0.0-beta.1 (2026-01-19)


### Bug Fixes

* audio playback issue ([#51](https://github.com/Alaydriem/bedrock-voice-chat/issues/51)) ([8b06085](https://github.com/Alaydriem/bedrock-voice-chat/commit/8b06085c281e65d1b550520748ee6b5dc4a82906))
* client side audio now works ([300d347](https://github.com/Alaydriem/bedrock-voice-chat/commit/300d347a533fe4510d681941210e4e5b5d7d78e8))
* migrate ncryptf to 0.5.0 to take advantage of cached::Cached ([550c129](https://github.com/Alaydriem/bedrock-voice-chat/commit/550c129c59d5fb19cb8a7f0da658b54b5c154a05)), closes [#5](https://github.com/Alaydriem/bedrock-voice-chat/issues/5)
* performance allocations ([ea719dd](https://github.com/Alaydriem/bedrock-voice-chat/commit/ea719dd43d03aca4d145ef97a2c15628da4ea374))
* tls port defaulting to :443 in client ([#80](https://github.com/Alaydriem/bedrock-voice-chat/issues/80)) ([936e01e](https://github.com/Alaydriem/bedrock-voice-chat/commit/936e01e16e4166d79d9e5693f23797bdb91e82b8)), closes [#79](https://github.com/Alaydriem/bedrock-voice-chat/issues/79) [#65](https://github.com/Alaydriem/bedrock-voice-chat/issues/65)


### Features

* add channel management + facilities ([#26](https://github.com/Alaydriem/bedrock-voice-chat/issues/26)) ([b64acb6](https://github.com/Alaydriem/bedrock-voice-chat/commit/b64acb6112d746a2efec17bcfb5954eb4486f1ca))
* add zigzag + varint encoding on QuicNetworkPacket and ([4a2498e](https://github.com/Alaydriem/bedrock-voice-chat/commit/4a2498e70bc093aac2fec7f7ca567c1144bdd822))
* adding client connect/disconnect events ([1772317](https://github.com/Alaydriem/bedrock-voice-chat/commit/1772317a0182b9bae0db55880a398843786c34f4))
* bvc server as a library ([#81](https://github.com/Alaydriem/bedrock-voice-chat/issues/81)) ([a57b7e1](https://github.com/Alaydriem/bedrock-voice-chat/commit/a57b7e12414b6c9748ca3d1d9fceb94f521d42bc)), closes [#75](https://github.com/Alaydriem/bedrock-voice-chat/issues/75)
* bwav recording output with timecode syncronization ([#40](https://github.com/Alaydriem/bedrock-voice-chat/issues/40)) ([906333f](https://github.com/Alaydriem/bedrock-voice-chat/commit/906333f923a189a6e6bd4ae74d21c5a5905578d7))
* disconnect client if there's a server / client mismatch ([26d7827](https://github.com/Alaydriem/bedrock-voice-chat/commit/26d78278a6d109a441f78cb69dc29a20f933315b))
* fabric java mod for cross platform servers ([#44](https://github.com/Alaydriem/bedrock-voice-chat/issues/44)) ([#45](https://github.com/Alaydriem/bedrock-voice-chat/issues/45)) ([4331f37](https://github.com/Alaydriem/bedrock-voice-chat/commit/4331f37d2d157735ba115e63f88cc8984c4a51e0))
* iOS builds ([#43](https://github.com/Alaydriem/bedrock-voice-chat/issues/43)) ([744bf72](https://github.com/Alaydriem/bedrock-voice-chat/commit/744bf729a16055d364d41a06267a848d1864cb2c)), closes [#21](https://github.com/Alaydriem/bedrock-voice-chat/issues/21)
* jitter buffer ([905885f](https://github.com/Alaydriem/bedrock-voice-chat/commit/905885fdee8d99ed1315da4d24f7da5b07ca7320))
* m4a + opus Audio Renderer ([#41](https://github.com/Alaydriem/bedrock-voice-chat/issues/41)) ([e3ef8e9](https://github.com/Alaydriem/bedrock-voice-chat/commit/e3ef8e93a65d5968e2b73fed815854326d60529e))
* migrate from RON to postcard ([b2a226b](https://github.com/Alaydriem/bedrock-voice-chat/commit/b2a226bc2e6979e5b5f08fff90edbef58aef6efa))
* ux improvements ([#36](https://github.com/Alaydriem/bedrock-voice-chat/issues/36)) ([aa4a765](https://github.com/Alaydriem/bedrock-voice-chat/commit/aa4a765c72a8c42edd07ce12fbbb9ce45ea6b4b9))
* websocket server ([#48](https://github.com/Alaydriem/bedrock-voice-chat/issues/48)) ([a0b67d7](https://github.com/Alaydriem/bedrock-voice-chat/commit/a0b67d7ef45038793085843e07adba84fbeb3cb1))
* working authentication to dashboard with stronghold + store ([cfe2cca](https://github.com/Alaydriem/bedrock-voice-chat/commit/cfe2ccad5fe96cb162291a2cbbbdc238a9b48844))
* working authentication to dashboard with stronghold + store ([00b9859](https://github.com/Alaydriem/bedrock-voice-chat/commit/00b9859a437e1ece41f172948260b834fb976a5a))
