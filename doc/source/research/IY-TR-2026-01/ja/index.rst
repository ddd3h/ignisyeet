:orphan:
:hreflang-en: https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/
:hreflang-ja: https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/ja/

.. meta::
   :description: IY-TR-2026-01（日本語版）。NASA WIND/NPARC の手順で、CAD から係数まで CFD を自動で実行する IgnisYeet v0.4.0 の数値不確かさを定量化できるかを問う技術報告。

IY-TR-2026-01: NASAの手順によるIgnisYeet v0.4.0のCFDの検証と妥当性確認
======================================================================

.. raw:: html

   <div class="iy">

   <header class="iy-hero iy-page-hero">
     <div class="iy-wrap">
       <div class="iy-hero-top"><p class="iy-crumb"><a href="../../">R&amp;D</a></p><a class="iy-lang" href="../" hreflang="en" lang="en" title="Read in English" aria-label="Read in English"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m5 8 6 6"/><path d="m4 14 6-6 2-3"/><path d="M2 5h12"/><path d="M7 2h1"/><path d="m22 22-5-10-5 10"/><path d="M14 18h6"/></svg></a></div>
       <p class="iy-badges"><span class="iy-badge">IY-TR-2026-01</span><span class="iy-badge">技術報告</span><span class="iy-badge">日本語</span></p>
       <p class="iy-page-title iy-report-title" role="presentation">NASAの手順によるIgnisYeet v0.4.0のCFDの検証と妥当性確認</p>
       <p class="iy-meta">西濱 大将 ／ 2026年10月</p>
       <p class="iy-meta">キーワード：検証と妥当性確認, 格子収束指標, SU2, ロケット空力, 不確かさの定量化</p>
       <p class="iy-actions"><a class="iy-btn iy-btn-primary" href="../IY-TR-2026-01-ja.pdf"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M12 11v6M9.5 14.5 12 17l2.5-2.5"/></svg>PDF</a><button type="button" class="iy-btn" id="iy-cite-open" aria-haspopup="dialog"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M9.6 6C6.5 6.9 4.5 9.6 4.5 13v5h6v-6H7.6c.2-1.9 1.4-3.4 3.2-4.1zm9 0c-3.1.9-5.1 3.6-5.1 7v5h6v-6h-2.9c.2-1.9 1.4-3.4 3.2-4.1z"/></svg>引用</button><button type="button" class="iy-btn" id="iy-share-open" aria-haspopup="dialog"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="M8.6 13.5l6.8 4M15.4 6.5l-6.8 4"/></svg>共有</button></p>
       <dialog class="iy-cite iy-share" id="iy-share" aria-labelledby="iy-share-title">
         <div class="iy-cite-head"><p class="iy-cite-title" id="iy-share-title">この報告を共有する（英語版のページ）</p><form method="dialog"><button class="iy-search-close" aria-label="閉じる">Esc</button></form></div>
         <div class="iy-cite-block">
           <div class="iy-share-link"><input type="text" id="iy-share-url" readonly aria-label="この報告へのリンク"><button type="button" class="iy-cite-copy" id="iy-share-copy">リンクをコピー</button></div>
           <ul class="iy-share-targets">
             <li><a id="iy-share-x" target="_blank" rel="noopener">X</a></li>
             <li><a id="iy-share-linkedin" target="_blank" rel="noopener">LinkedIn</a></li>
             <li><a id="iy-share-facebook" target="_blank" rel="noopener">Facebook</a></li>
             <li><a id="iy-share-mail">メール</a></li>
           </ul>
         </div>
       </dialog>
       <dialog class="iy-cite" id="iy-cite" aria-labelledby="iy-cite-title">
         <div class="iy-cite-head"><p class="iy-cite-title" id="iy-cite-title">この報告を引用する（英語版）</p><form method="dialog"><button class="iy-search-close" aria-label="閉じる">Esc</button></form></div>
         <div class="iy-cite-block">
           <div class="iy-cite-label"><span>BibTeX</span><span><a href="../IY-TR-2026-01.bib" download>.bib</a><button type="button" class="iy-cite-copy" data-target="iy-cite-bib">コピー</button></span></div>
           <pre id="iy-cite-bib" lang="en">@techreport{nishihama2026iytr202601,
     author      = {Nishihama, Daisuke},
     title       = {{NASA} {CFD} verification and validation of {IgnisYeet} v0.4.0},
     institution = {IgnisYeet},
     type        = {Technical report},
     number      = {IY-TR-2026-01},
     year        = {2026},
     month       = oct,
     url         = {https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/}
   }</pre>
         </div>
         <div class="iy-cite-block">
           <div class="iy-cite-label"><span>テキスト</span><button type="button" class="iy-cite-copy" data-target="iy-cite-text">コピー</button></div>
           <p id="iy-cite-text" lang="en">Nishihama, D. (2026). <i>NASA CFD verification and validation of IgnisYeet v0.4.0</i> (Technical report IY-TR-2026-01). IgnisYeet. https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/</p>
         </div>
       </dialog>
     </div>
   </header>

   <div class="iy-wrap iy-paper-layout">

.. raw:: html
   :file: ../paper-ja.html

.. raw:: html

   </div>
   <script>
   (function () {
     var toc = document.querySelector(".iy-toc");
     if (!toc) { return; }
     var details = toc.querySelector("details");
     if (matchMedia("(max-width: 66.99em)").matches) { details.open = false; }
     var links = {};
     toc.querySelectorAll("a").forEach(function (a) { links[a.getAttribute("href").slice(1)] = a; });
     var heads = Array.prototype.filter.call(document.querySelectorAll(".iy-paper h2[id], .iy-paper h3[id]"), function (h) { return links[h.id]; });
     var current = null;
     var update = function () {
       var active = heads[0];
       for (var i = 0; i < heads.length; i++) { if (heads[i].getBoundingClientRect().top < 120) { active = heads[i]; } else { break; } }
       if (active === current) { return; }
       if (current) { links[current.id].removeAttribute("aria-current"); }
       current = active;
       var a = links[current.id];
       a.setAttribute("aria-current", "location");
       var box = toc.getBoundingClientRect(), r = a.getBoundingClientRect();
       if (r.top < box.top || r.bottom > box.bottom) { a.scrollIntoView({ block: "nearest" }); }
     };
     addEventListener("scroll", update, { passive: true });
     update();
     toc.addEventListener("click", function (e) { if (e.target.closest("a") && matchMedia("(max-width: 66.99em)").matches) { details.open = false; } });
   })();
   (function () {
     var dialog = document.getElementById("iy-share"), opener = document.getElementById("iy-share-open");
     // Both language pages share the English report.
     var url = "https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/", title = "NASA CFD verification and validation of IgnisYeet v0.4.0 (IY-TR-2026-01)";
     var u = encodeURIComponent(url), t = encodeURIComponent(title);
     document.getElementById("iy-share-url").value = url;
     document.getElementById("iy-share-x").href = "https://x.com/intent/post?url=" + u + "&text=" + t;
     document.getElementById("iy-share-linkedin").href = "https://www.linkedin.com/sharing/share-offsite/?url=" + u;
     document.getElementById("iy-share-facebook").href = "https://www.facebook.com/sharer/sharer.php?u=" + u;
     document.getElementById("iy-share-mail").href = "mailto:?subject=" + t + "&body=" + u;
     opener.addEventListener("click", function () {
       // Phones and some desktops have a system share sheet; use it when there is one.
       if (navigator.share) { navigator.share({ title: title, url: url }).catch(function () {}); return; }
       if (typeof dialog.showModal === "function") { dialog.showModal(); }
     });
     dialog.addEventListener("click", function (e) { if (e.target === dialog) { dialog.close(); } });
     var copy = document.getElementById("iy-share-copy");
     copy.addEventListener("click", function () {
       var done = function () { copy.textContent = "コピーしました"; setTimeout(function () { copy.textContent = "リンクをコピー"; }, 1600); };
       if (navigator.clipboard && navigator.clipboard.writeText) { navigator.clipboard.writeText(url).then(done); }
       else { document.getElementById("iy-share-url").select(); }
     });
   })();
   (function () {
     var dialog = document.getElementById("iy-cite"), opener = document.getElementById("iy-cite-open");
     if (!dialog || typeof dialog.showModal !== "function") { opener.hidden = true; return; }
     opener.addEventListener("click", function () { dialog.showModal(); });
     dialog.addEventListener("click", function (e) { if (e.target === dialog) { dialog.close(); } });
     dialog.querySelectorAll(".iy-cite-copy").forEach(function (btn) {
       btn.addEventListener("click", function () {
         var text = document.getElementById(btn.getAttribute("data-target")).textContent;
         var done = function () { btn.textContent = "コピーしました"; setTimeout(function () { btn.textContent = "コピー"; }, 1600); };
         if (navigator.clipboard && navigator.clipboard.writeText) { navigator.clipboard.writeText(text).then(done); }
       });
     });
   })();
   </script>

   </div>
