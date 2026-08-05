import type { SelectOption } from "$radial/core/controllers/SelectControl";
import type { AudioDevice } from "../../bindings/AudioDevice";

/**
 * Devices for the picker, gathered under the host that offers them.
 *
 * The same interface under ASIO and under WASAPI is a different latency, not a duplicate.
 */
export class AudioDeviceGroups {
    // WASAPI first: it is the common case.
    private static readonly HOSTS: readonly (readonly [string, string])[] = [
        ["Wasapi", "WASAPI"],
        ["Asio", "ASIO"],
    ];

    static of(devices: readonly AudioDevice[]): readonly SelectOption[] {
        const present = this.HOSTS.map(
            ([host, label]) =>
                [label, devices.filter((device) => device.host === host)] as const,
        ).filter(([, group]) => group.length > 0);

        if (present.length < 2) {
            return present.flatMap(([, group]) => group.map((device) => device.display_name));
        }

        return present.flatMap(([label, group]) => [
            { section: label },
            ...group.map((device) => device.display_name),
        ]);
    }
}
