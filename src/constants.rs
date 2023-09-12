pub const BLOBINIT_SCRIPT: &str = "
	// Adds an URL.getFromObjectURL( <blob:// URI> ) method
	// returns the original object (<Blob> or <MediaSource>) the URI points to or null
	(() => {
		// overrides URL methods to be able to retrieve the original blobs later on
		const old_create = URL.createObjectURL;
		const old_revoke = URL.revokeObjectURL;
		Object.defineProperty(URL, 'createObjectURL', {
			get: () => storeAndCreate
		});
		Object.defineProperty(URL, 'revokeObjectURL', {
			get: () => forgetAndRevoke
		});
		Object.defineProperty(URL, 'getFromObjectURL', {
			get: () => getBlob
		});
		Object.defineProperty(URL, 'getObjectURLDict', {
			get: () => getDict
		});
		Object.defineProperty(URL, 'clearURLDict', {
			get: () => clearDict
		});
		const dict = {};

		function storeAndCreate(blob) {
			const url = old_create(blob); // let it throw if it has to
			dict[url] = blob;
			console.log(url)
			console.log(blob)
			return url
		}

		function forgetAndRevoke(url) {
			console.log(`revoke ${url}`)
			old_revoke(url);
		}

		function getBlob(url) {
			return dict[url] || null;
		}

		function getDict() {
			return dict;
		}

		function clearDict() {
			dict = {};
		}
	})();
";

pub const DEV_TOOLS_HTML: &str = "
	<style>
			#devtools {
					position: relative;
					top: 0;
					left: 0;
					width: 100%;
					height: 20px;
					background-color: #0f0f0f;
					color: #fff;
					display: flex;
					z-index: 9999;
			}
			#devtools button {
					background-color: #0f0f0f;
					color: #fff;
					border: 1px solid #404040;
					padding: 2px 10px;
					font-size: 10px;
					cursor: pointer;
			}
			#devtools button:hover {
					background-color: #404040;
			}
			#devtools button:active {
					background-color: #0f0f0f;
			}
	</style>
	<div id='devtools'>
			<button onclick=\"window.pywry.devtools()\">DevTools</button>
	</div>
";

/// Pywry Window function script that is injected into the HTML to allow
/// communication between the HTML and the Rust backend.
pub const PYWRY_WINDOW_SCRIPT: &str = "
	window.pywry = {
		result: function (result) {
			window.ipc.postMessage(`#PYWRY_RESULT:${result}`);
		},
		open_file: function (file_path) {
			window.ipc.postMessage(`#OPEN_FILE:${file_path}`);
		},
		devtools: function () {
			window.ipc.postMessage('#DEVTOOLS');
		},
	};
";

// Add keyboard shortcuts for copy and paste (fixes Mac OS)
pub const MACOS_COPY_PASTE_SCRIPT: &str = "
	try {
		window.addEventListener('keydown', (e) => {
			if (e.key.toLowerCase() === 'c' && (e.ctrlKey || e.metaKey)) {
				e.preventDefault();
				document.execCommand('copy');
			}
			if (e.key.toLowerCase() === 'v' && (e.ctrlKey || e.metaKey)) {
				e.preventDefault();
				document.execCommand('paste');
			}
		});
	} catch (error) {
		console.log(error);
	}
";

/// Plotly script that is injected into the HTML to render the plot
/// headless and then send the image back to the Python backend.
pub const HEADLESS_HTML: &str = "
<html>
	<head>
		<meta charset='utf-8' />
		<meta name='viewport' content='width=device-width, initial-scale=1' />
		<script src='https://cdn.plot.ly/plotly-2.21.0.min.js'></script>
		<style>
			html,
			body {
				margin: 0;
				padding: 0;
				overflow: hidden;
			}
		</style>
	</head>
	<body>
	</body>
</html>
";

pub const PLOTLY_RENDER_JS: &str = "
function plotly_render(info) {
	const opts = {};
	try {
		const figure = info.figure;
		const defaultConfig = {
			plotGlPixelRatio: (info.scale || 2) * 2,
		};
		const config = Object.assign(defaultConfig, figure.config);

		const imgOpts = {
			format: info.format || 'png',
			width: info.width,
			height: info.height,
			scale: info.scale,
			imageDataOnly: info.format !== 'svg',
		};

		opts.figure = { ...figure, config: config };
		opts.imgOpts = imgOpts;
	} catch (err) {
		return window.pywry.result(err);
	}
	try {
		Plotly.toImage(opts.figure, opts.imgOpts).then(function (
			imageData
		) {
			return window.pywry.result(imageData);
		});
	} catch (err_1) {
		return window.pywry.result(err_1);
	}
	return true;
}
";
