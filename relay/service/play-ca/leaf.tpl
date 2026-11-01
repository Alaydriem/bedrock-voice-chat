{
  "subject": { "commonName": {{ toJson .Token.sub }} },
  "sans": {{ toJson .SANs }},
  "keyUsage": ["digitalSignature"],
  "extKeyUsage": ["serverAuth"]
}
