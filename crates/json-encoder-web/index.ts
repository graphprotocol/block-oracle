// Note that a dynamic `import` statement here is required due to
// webpack/webpack#6615, but in theory `import { greet } from './pkg';`
// will work here one day as well!
import * as wasm from './pkg';
import * as copy from 'copy-to-clipboard';
import * as monaco from 'monaco-editor';

self.MonacoEnvironment = {
	getWorkerUrl: function (moduleId, label) {
		return './json.worker.bundle.js';
	}
};

const samplePayload = `[
	{
		"add": [
			"A"
		],
		"message": "RegisterNetworks",
		"remove": []
	}
]
`;

var editor = monaco.editor.create(document.getElementById('container'), {
	value: samplePayload,
	language: 'json',
	minimap: {
		enabled: false
	},
	theme: 'vs-light'
});

document.getElementById('compile-button').onclick = function () {
	console.log('button was clicked');
	let input = editor.getValue();
	let compiled = wasm.compile(input, true);
	(<HTMLInputElement>document.getElementById('compiled')).value = compiled;
};

document.getElementById('copy-to-clipboard').onclick = function () {
	let compiled = (<HTMLInputElement>document.getElementById('compiled')).value;
	copy(compiled);
};

document.getElementById('clear-all').onclick = function () {
	editor.setValue('');
	(<HTMLFormElement>document.getElementById("form")).reset();
}
