/** @type {import('tailwindcss').Config} */
module.exports = {
	prefix: 'tw-',
	important: false,
	content: [
		"./*.html",
		"./*.js",
	],
	darkMode: 'class',
	theme: {
		extend: {
			colors: {
				primary: "#28bae1",
				secondary: "#6950e9",
				accent: "#3bd869",
			}
		},
	},
	plugins: [],
}

