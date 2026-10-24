use std::net::IpAddr;
use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait,
    PaginatorTrait, QueryFilter,
};

use crate::entity::dns_record;

use super::cloudflare_api::CloudflareApi;
use super::error::DnsError;

// Writes assigned names into the zone, and remembers what it wrote.
//
// The ledger is not the zone: Cloudflare is authoritative. It exists so cleanup can
// delete by record id rather than by matching content, and so an interrupted publish
// leaves no record nobody can identify.
pub struct ZoneWriter {
    conn: Arc<DatabaseConnection>,
    api: CloudflareApi,
    zone: String,
}

impl ZoneWriter {
    const TXT: &'static str = "TXT";
    const A: &'static str = "A";
    const AAAA: &'static str = "AAAA";

    pub fn new(conn: Arc<DatabaseConnection>, api: CloudflareApi, zone: String) -> Self {
        Self { conn, api, zone }
    }

    pub async fn publish_txt(&self, name: &str, value: &str) -> Result<(), DnsError> {
        let fqdn = self.challenge_fqdn(name);
        self.create(Self::TXT, &fqdn, value).await
    }

    // Every challenge record for the name, not the most recent one. An overlapping
    // retry leaves two, and a cleanup that removed only one would leave the zone
    // carrying a stale authorization.
    pub async fn cleanup_txt(&self, name: &str) -> Result<(), DnsError> {
        let fqdn = self.challenge_fqdn(name);
        self.delete_where(dns_record::Column::Name.eq(fqdn)).await
    }

    // The address record is replaced rather than added to. A name resolves to one
    // server, and two records would round-robin between an operator's current address
    // and one they moved off. The family picks the record type: an IPv6 address in an A
    // record is refused by the zone, so it is published as AAAA.
    pub async fn publish_address(&self, name: &str, address: IpAddr) -> Result<(), DnsError> {
        let kind = match address {
            IpAddr::V4(_) => Self::A,
            IpAddr::V6(_) => Self::AAAA,
        };
        self.withdraw_address(name).await?;
        self.create(kind, &self.address_fqdn(name), &address.to_string())
            .await
    }

    // The address record alone, leaving any challenge in flight. Used when an operator
    // declares a new address: the old record points at a host they have said they
    // moved off, and the new one is not published until it is verified.
    pub async fn withdraw_address(&self, name: &str) -> Result<(), DnsError> {
        self.delete_where(dns_record::Column::Name.eq(self.address_fqdn(name)))
            .await
    }

    // Challenge records still outstanding for this name. Read before publishing
    // another, so a node cannot fill the zone with them.
    pub async fn open_challenges(&self, name: &str) -> Result<u64, DnsError> {
        Ok(dns_record::Entity::find()
            .filter(dns_record::Column::Name.eq(self.challenge_fqdn(name)))
            .count(self.conn.as_ref())
            .await?)
    }

    // Every challenge record created before `cutoff`, across the zone. A DNS-01
    // challenge is answered within minutes, and nothing on the wire tells the relay
    // when one is done, so age is the only signal that a record is finished with.
    pub async fn prune_challenges_before(&self, cutoff: i64) -> Result<(), DnsError> {
        self.delete_where(
            dns_record::Column::RecordType
                .eq(Self::TXT)
                .and(dns_record::Column::CreatedAt.lt(cutoff)),
        )
        .await
    }

    // Everything the relay has written for this name — address and any challenge
    // left behind — so a suspended registration stops resolving entirely.
    pub async fn withdraw(&self, name: &str) -> Result<(), DnsError> {
        self.delete_where(
            dns_record::Column::Name
                .eq(self.address_fqdn(name))
                .or(dns_record::Column::Name.eq(self.challenge_fqdn(name))),
        )
        .await
    }

    // The name a server is actually reachable at.
    //
    // Public because it is the boundary between the two forms this name takes: a bare
    // label everywhere inside the registry — it keys the registration, the ledger, the
    // retired list and the issuance budget — and a fully qualified name the moment it
    // leaves for a server, which needs something a resolver and a certificate authority
    // will accept.
    pub fn address_fqdn(&self, name: &str) -> String {
        format!("{name}.{}", self.zone)
    }

    // The label for a fully qualified name in this zone, or `None` for a name that is
    // not in it. A server sends back what it was given, so the comparison has to happen
    // in one direction or the other; stripping is what keeps the stored label the only
    // internal identity.
    pub fn label_of(&self, fqdn: &str) -> Option<String> {
        fqdn.strip_suffix(&format!(".{}", self.zone))
            .filter(|label| !label.is_empty() && !label.contains('.'))
            .map(str::to_string)
    }

    fn challenge_fqdn(&self, name: &str) -> String {
        format!("_acme-challenge.{name}.{}", self.zone)
    }

    async fn create(&self, kind: &str, fqdn: &str, content: &str) -> Result<(), DnsError> {
        let record_id = self.api.create(kind, fqdn, content).await?;

        dns_record::ActiveModel {
            record_id: ActiveValue::Set(record_id),
            name: ActiveValue::Set(fqdn.to_string()),
            record_type: ActiveValue::Set(kind.to_string()),
            content: ActiveValue::Set(content.to_string()),
            created_at: ActiveValue::Set(Self::now()),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(())
    }

    // The zone is changed before the ledger row is dropped. The other order would
    // leave a record nothing names if the process died between them, and an
    // unnameable record can only be found by listing the whole zone.
    async fn delete_where(&self, filter: sea_orm::sea_query::SimpleExpr) -> Result<(), DnsError> {
        let rows = dns_record::Entity::find()
            .filter(filter)
            .all(self.conn.as_ref())
            .await?;

        for row in rows {
            self.api.delete(&row.record_id).await?;
            dns_record::Entity::delete_by_id(row.record_id)
                .exec(self.conn.as_ref())
                .await?;
        }

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
