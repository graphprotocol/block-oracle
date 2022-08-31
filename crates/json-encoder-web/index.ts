import * as wasm from './pkg';
import * as copy from 'copy-to-clipboard';
// https://github.com/microsoft/monaco-editor/issues/2874
import { editor as monacoEditor } from 'monaco-editor/esm/vs/editor/editor.api'

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

var editor = monacoEditor.create(document.getElementById('container'), {
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
