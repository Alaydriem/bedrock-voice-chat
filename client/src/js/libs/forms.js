//  Forms Libraries
import * as FilePond from "filepond"; // @see https://pqina.nl/filepond/
import FilePondPluginImagePreview from "filepond-plugin-image-preview"; // @see https://pqina.nl/filepond/docs/api/plugins/image-preview/
import Quill from "quill/dist/quill.min"; // @see https://quilljs.com/
import flatpickr from "flatpickr"; // @see https://flatpickr.js.org/
import Tom from "tom-select/dist/js/tom-select.complete.min"; // @see https://tom-select.js.org/
import Cleave from "cleave.js/dist/cleave.min"; // @see https://github.com/nosir/cleave.js

// Register plugin image preview for filepond
FilePond.registerPlugin(FilePondPluginImagePreview);

window.FilePond = FilePond;
window.flatpickr = flatpickr;
window.Quill = Quill;
window.Tom = Tom;
window.Cleave = Cleave;
