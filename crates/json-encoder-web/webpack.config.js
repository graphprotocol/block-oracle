const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
	entry: {
		app: './index.ts',
		'json.worker': 'monaco-editor/esm/vs/language/json/json.worker'
	},
	output: {
		path: path.resolve(__dirname, 'dist'),
		globalObject: 'self',
		filename: '[name].bundle.js',
	},
	plugins: [
		new HtmlWebpackPlugin({
			template: 'index.html'
		}),
		new WasmPackPlugin({
			crateDirectory: path.resolve(__dirname, ".")
		}),
		// Have this example work in Edge which doesn't ship `TextEncoder` or
		// `TextDecoder` at this time.
		new webpack.ProvidePlugin({
			TextDecoder: ['text-encoding', 'TextDecoder'],
			TextEncoder: ['text-encoding', 'TextEncoder']
		})
	],
	mode: 'development',
	experiments: {
		asyncWebAssembly: true
	},
	module: {
		rules: [
			{
				test: /\.css$/,
				use: ['style-loader', 'css-loader']
			},
			{
				// https://stackoverflow.com/q/71674567
				test: /\.ttf$/,
				type: 'asset/resource'
			},
			{
				test: /\.ts?$/,
				use: 'ts-loader',
				exclude: /node_modules/,
			},
		]
	},
	resolve: {
		extensions: ['', '.js', '.jsx', '.css', '.ts']
	}
};