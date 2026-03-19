// initialization

// ---- Version config (update this when releasing) ----
const BVC_VERSION = "1.0.0-beta.9"
const BVC_MODS_VERSION = "mods-v1.0.0-beta.9"

// Populate all version references on page load
document.querySelectorAll("[data-bvc-version]").forEach(el => {
    el.textContent = el.textContent.replace("{{VERSION}}", BVC_VERSION)
})
document.querySelectorAll("[data-bvc-version-href]").forEach(el => {
    el.setAttribute("href", el.getAttribute("href").replace("{{VERSION}}", BVC_VERSION))
})
document.querySelectorAll("[data-bvc-mods-href]").forEach(el => {
    el.setAttribute("href", el.getAttribute("href").replace("{{MODS_VERSION}}", BVC_MODS_VERSION))
})

const RESPONSIVE_WIDTH = 1024

let headerWhiteBg = false
let isHeaderCollapsed = window.innerWidth < RESPONSIVE_WIDTH
const collapseBtn = document.getElementById("collapse-btn")
const collapseHeaderItems = document.getElementById("collapsed-header-items")



function onHeaderClickOutside(e) {

    if (!collapseHeaderItems.contains(e.target)) {
        toggleHeader()
    }

}


function toggleHeader() {
    if (isHeaderCollapsed) {
        collapseHeaderItems.classList.add("menu-open")
        collapseBtn.classList.remove("bi-list")
        collapseBtn.classList.add("bi-x")
        isHeaderCollapsed = false

        setTimeout(() => window.addEventListener("click", onHeaderClickOutside), 1)

    } else {
        collapseHeaderItems.classList.remove("menu-open")
        collapseBtn.classList.remove("bi-x")
        collapseBtn.classList.add("bi-list")
        isHeaderCollapsed = true
        window.removeEventListener("click", onHeaderClickOutside)

    }
}

function responsive() {
    if (window.innerWidth > RESPONSIVE_WIDTH) {
        collapseHeaderItems.classList.remove("menu-open")
        isHeaderCollapsed = true
    } else {
        isHeaderCollapsed = true
    }
}

window.addEventListener("resize", responsive)


/**
 * Animations
 */

gsap.registerPlugin(ScrollTrigger)


gsap.to(".reveal-up", {
    opacity: 0,
    y: "100%",
})

// ------------- reveal section animations ---------------

const sections = gsap.utils.toArray("section")

sections.forEach((sec) => {

    const revealUptimeline = gsap.timeline({paused: true, 
                                            scrollTrigger: {
                                                            trigger: sec,
                                                            start: "10% 80%", // top of trigger hits the top of viewport
                                                            end: "20% 90%",
                                                            // markers: true,
                                                            // scrub: 1,
                                                        }})

    revealUptimeline.to(sec.querySelectorAll(".reveal-up"), {
        opacity: 1,
        duration: 0.8,
        y: "0%",
        stagger: 0.2,
    })


})
