const onLoad = () => {
    new Popper("#engine-selector-dropdown-wrapper", ".popper-ref", ".popper-root", {
        placement: "bottom-start",
        modifiers: [
            {
                name: "offset",
                options: {
                    offset: [0, 4],
                },
            },
        ],
    });
};

window.addEventListener("app:mounted", onLoad, { once: true });
