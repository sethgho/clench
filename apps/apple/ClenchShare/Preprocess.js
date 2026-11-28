// Safari runs this in the shared page before the share extension opens and
// hands the result to the extension under
// NSExtensionJavaScriptPreprocessingResultsKey. Sending the rendered page
// lets clench read pages that need JavaScript, a login or a real browser.
var ClenchPreprocessing = function () {};

ClenchPreprocessing.prototype = {
  run: function (args) {
    var selection = "";
    try {
      selection = String(window.getSelection ? window.getSelection() : "");
    } catch (e) {}
    args.completionFunction({
      url: document.URL,
      title: document.title,
      html: document.documentElement ? document.documentElement.outerHTML : "",
      selection: selection,
    });
  },
  finalize: function () {},
};

var ExtensionPreprocessingJS = new ClenchPreprocessing();
