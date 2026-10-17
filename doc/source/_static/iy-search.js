/* Search dialog for the top bar of the landing page and the research page.
   Uses Sphinx's own search index and scoring, so the results match search.html. */
(function () {
  var dialog = document.getElementById("iy-search");
  var opener = document.getElementById("iy-search-open");
  if (!dialog || !opener || typeof dialog.showModal !== "function") { return; }
  var input = dialog.querySelector("input");
  var list = dialog.querySelector(".iy-search-results");
  var status = dialog.querySelector(".iy-search-status");
  var more = dialog.querySelector(".iy-search-more");
  var root = document.documentElement.dataset.content_root || "";
  var loading = null, timer = null;

  var load = function () {
    if (loading) { return loading; }
    var add = function (src) {
      return new Promise(function (resolve, reject) {
        var s = document.createElement("script");
        s.src = root + src; s.onload = resolve; s.onerror = reject;
        document.body.appendChild(s);
      });
    };
    loading = add("_static/language_data.js")
      .then(function () { return add("_static/searchtools.js"); })
      .then(function () { return add("searchindex.js"); });
    return loading;
  };

  var render = function () {
    var q = input.value.trim();
    list.textContent = "";
    more.href = root + "search.html" + (q ? "?q=" + encodeURIComponent(q) : "");
    if (!q) { status.textContent = ""; return; }
    if (typeof Search === "undefined" || !Search.hasIndex()) { status.textContent = "索引を読み込み中…"; return; }
    var parsed = Search._parseQuery(q);
    var results = Search._performSearch.apply(null, parsed).reverse();
    status.textContent = results.length ? results.length + " 件" : "見つからない";
    results.slice(0, 30).forEach(function (r) {
      var li = list.appendChild(document.createElement("li"));
      var a = li.appendChild(document.createElement("a"));
      a.href = root + r[0] + DOCUMENTATION_OPTIONS.LINK_SUFFIX + "?highlight=" + encodeURIComponent(q) + r[2];
      a.textContent = r[1];
    });
  };

  opener.addEventListener("click", function (e) {
    e.preventDefault();
    dialog.showModal();
    input.select();
    load().then(render, function () { status.textContent = "索引を読み込めなかった"; });
  });
  input.addEventListener("input", function () { clearTimeout(timer); timer = setTimeout(render, 120); });
  input.addEventListener("keydown", function (e) {
    var first = list.querySelector("a");
    if (e.key === "Enter" && first) { e.preventDefault(); location.href = first.href; }
    if (e.key === "ArrowDown" && first) { e.preventDefault(); first.focus(); }
    if (e.key === "Escape") { e.preventDefault(); dialog.close(); }
  });
  list.addEventListener("keydown", function (e) {
    var li = document.activeElement.parentElement;
    var next = e.key === "ArrowDown" ? li.nextElementSibling : e.key === "ArrowUp" ? li.previousElementSibling : undefined;
    if (next === undefined) { return; }
    e.preventDefault();
    if (next) { next.firstChild.focus(); } else if (e.key === "ArrowUp") { input.focus(); }
  });
  dialog.addEventListener("click", function (e) { if (e.target === dialog) { dialog.close(); } });
})();
