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
