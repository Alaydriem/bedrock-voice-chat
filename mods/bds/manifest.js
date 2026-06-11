const config = require('./manifest.config');

/**
 * Generates concrete behavior- and resource-pack manifests for a given variant.
 *
 * Applies the variant's UUID set, the encoded version (header + modules + pack
 * dependencies), literal name/description overrides, module stripping, and wires
 * the bidirectional BP <-> RP pack dependencies to that variant's own UUIDs.
 */
class ManifestBuilder {
  constructor(encoded, semver) {
    this.encoded = encoded;
    this.semver = semver;
  }

  resolveDescription(description) {
    return description.replace('{version}', this.semver);
  }

  bp(variantKey) {
    const variant = config.VARIANTS[variantKey];
    const uuids = config.UUIDS[variantKey];
    const version = this.encoded.array;

    const dependencies = [
      { version, uuid: uuids.rp.header },
      ...config.BP_MODULE_DEPENDENCIES.filter(
        (dep) => !variant.stripModules.includes(dep.module_name)
      ),
    ];

    return {
      format_version: 2,
      header: {
        name: this.resolveDescription(variant.name),
        description: this.resolveDescription(variant.description),
        uuid: uuids.bp.header,
        version,
        min_engine_version: config.BP_MIN_ENGINE,
      },
      modules: [
        { type: 'data', uuid: uuids.bp.data, version },
        {
          type: 'script',
          uuid: uuids.bp.script,
          version,
          entry: config.SCRIPT_ENTRY,
          language: 'javascript',
        },
      ],
      dependencies,
      metadata: config.METADATA,
    };
  }

  rp(variantKey) {
    const variant = config.VARIANTS[variantKey];
    const uuids = config.UUIDS[variantKey];
    const version = this.encoded.array;

    return {
      format_version: 2,
      header: {
        name: this.resolveDescription(variant.name),
        description: this.resolveDescription(variant.description),
        uuid: uuids.rp.header,
        version,
        min_engine_version: config.RP_MIN_ENGINE,
      },
      modules: [{ type: 'resources', uuid: uuids.rp.resource, version }],
      dependencies: [{ version, uuid: uuids.bp.header }],
      metadata: config.METADATA,
    };
  }
}

module.exports = { ManifestBuilder };
