const onLoad = () => {
    const dropdownConfig = {
        placement: "bottom-start",
        modifiers: [
            {
                name: "offset",
                options: {
                    offset: [0, 4],
                },
            },
        ],
    };

    new Popper("#navigation-dropdown-1", ".popper-ref", ".popper-root", dropdownConfig);
    new Popper("#navigation-dropdown-2", ".popper-ref", ".popper-root", dropdownConfig);
    new Popper("#navigation-dropdown-3", ".popper-ref", ".popper-root", dropdownConfig);
    new Popper("#navigation-dropdown-4", ".popper-ref", ".popper-root", dropdownConfig);
    new Popper("#navigation-dropdown-5", ".popper-ref", ".popper-root", dropdownConfig);
}

window.addEventListener("app:mounted", onLoad, { once: true });
