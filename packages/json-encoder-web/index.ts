import * as wasm from './pkg';
import * as copy from 'copy-to-clipboard';
import notie from 'notie';
import { editor as monacoEditor } from 'monaco-editor/esm/vs/editor/editor.api'

require('notie/dist/notie.min.css');

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

// https://github.com/microsoft/monaco-editor/issues/2874
self.MonacoEnvironment = {
	getWorkerUrl: function (moduleId, label) {
		return './json.worker.bundle.js';
	}
};

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

	try {
		let compiled = wasm.compile(input, true);
		(<HTMLInputElement>document.getElementById('compiled')).value = toHexString(compiled);
	}
	catch (e: any) {
		notie.alert({ text: (<string>e), time: 2, type: 'error' });
	}
};

document.getElementById('copy-to-clipboard').onclick = function () {
	let compiled = (<HTMLInputElement>document.getElementById('compiled')).value;
	notie.alert({ text: `Copied ${compiled.length} characters to the clipboard.`, time: 1, type: 'success' });
	copy(compiled);
};

document.getElementById('clear-all').onclick = function () {
	editor.setValue('');
	(<HTMLFormElement>document.getElementById("form")).reset();
}


function toHexString(byteArray: Uint8Array): string {
	var s = '';
	byteArray.forEach(function (byte) {
		s += ('0' + (byte & 0xFF).toString(16)).slice(-2);
	});
	return s;
}