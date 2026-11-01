{
  "subject": {{ toJson .Subject }},
  "issuer": {{ toJson .Subject }},
  "keyUsage": ["certSign", "crlSign"],
  "basicConstraints": { "isCA": true, "maxPathLen": 1 },
  "nameConstraints": {
    "critical": true,
    "permittedDNSDomains": ["play.bedrockvc.stream"],
    "excludedIPRanges": ["0.0.0.0/0", "::/0"]
  }
}
