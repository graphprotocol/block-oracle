#!/usr/bin/env node
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function buildWasm() {
  console.log('🔨 Building WASM module...');
  
  // Clean previous build
  if (fs.existsSync('pkg')) {
    fs.rmSync('pkg', { recursive: true, force: true });
  }
  fs.mkdirSync('pkg');
  
  try {
    // Compile Rust to WASM
    console.log('📦 Compiling Rust to WASM...');
    execSync('cargo build --target wasm32-unknown-unknown --release', {
      stdio: 'inherit'
    });
    
    // Generate bindings
    console.log('🔗 Generating bindings...');
    execSync(`wasm-bindgen \
      target/wasm32-unknown-unknown/release/json_encoder_web.wasm \
      --out-dir pkg \
      --target bundler \
      --no-typescript`, {
      stdio: 'inherit'
    });
    
    console.log('✅ WASM build complete!');
  } catch (error) {
    console.error('❌ Build failed:', error.message);
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  buildWasm();
}

module.exports = { buildWasm };