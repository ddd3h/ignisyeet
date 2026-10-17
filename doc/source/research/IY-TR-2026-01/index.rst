:orphan:
:hreflang-en: https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/
:hreflang-ja: https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/ja/

.. meta::
   :description: IY-TR-2026-01. NASA CFD verification and validation of IgnisYeet v0.4.0: whether the NASA WIND/NPARC procedure can quantify the numerical uncertainty of an automated CAD-to-coefficient CFD pipeline.
   :citation_title: NASA CFD verification and validation of IgnisYeet v0.4.0
   :citation_author: Nishihama, Daisuke
   :citation_publication_date: 2026/10
   :citation_technical_report_institution: IgnisYeet
   :citation_technical_report_number: IY-TR-2026-01
   :citation_language: en
   :citation_pdf_url: https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/IY-TR-2026-01.pdf

IY-TR-2026-01: NASA CFD verification and validation of IgnisYeet v0.4.0
========================================================================

.. raw:: html

   <div class="iy">

   <header class="iy-hero iy-page-hero">
     <div class="iy-wrap">
       <p class="iy-crumb"><a href="../">R&amp;D</a></p>
       <p class="iy-badges"><span class="iy-badge">IY-TR-2026-01</span><span class="iy-badge">Technical report</span><span class="iy-badge">English</span></p>
       <p class="iy-page-title iy-report-title" role="presentation" lang="en">NASA CFD verification and validation of IgnisYeet v0.4.0</p>
       <p class="iy-meta" lang="en">Daisuke Nishihama ／ October 2026</p>
       <p class="iy-meta" lang="en">Keywords: verification and validation, grid convergence index, SU2, rocket aerodynamics, uncertainty quantification</p>
       <p class="iy-actions"><a class="iy-btn iy-btn-primary" href="IY-TR-2026-01.pdf"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M12 11v6M9.5 14.5 12 17l2.5-2.5"/></svg>PDF</a><button type="button" class="iy-btn" id="iy-cite-open" aria-haspopup="dialog"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M9.6 6C6.5 6.9 4.5 9.6 4.5 13v5h6v-6H7.6c.2-1.9 1.4-3.4 3.2-4.1zm9 0c-3.1.9-5.1 3.6-5.1 7v5h6v-6h-2.9c.2-1.9 1.4-3.4 3.2-4.1z"/></svg>Cite</button><button type="button" class="iy-btn" id="iy-share-open" aria-haspopup="dialog"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="M8.6 13.5l6.8 4M15.4 6.5l-6.8 4"/></svg>Share</button><a class="iy-btn" href="ja/" hreflang="ja" lang="ja"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z"/></svg>日本語</a></p>
       <dialog class="iy-cite iy-share" id="iy-share" aria-labelledby="iy-share-title" lang="en">
         <div class="iy-cite-head"><p class="iy-cite-title" id="iy-share-title">Share this report</p><form method="dialog"><button class="iy-search-close" aria-label="Close">Esc</button></form></div>
         <div class="iy-cite-block">
           <div class="iy-share-link"><input type="text" id="iy-share-url" readonly aria-label="Link to this report"><button type="button" class="iy-cite-copy" id="iy-share-copy">Copy link</button></div>
           <ul class="iy-share-targets">
             <li><a id="iy-share-x" target="_blank" rel="noopener">X</a></li>
             <li><a id="iy-share-linkedin" target="_blank" rel="noopener">LinkedIn</a></li>
             <li><a id="iy-share-facebook" target="_blank" rel="noopener">Facebook</a></li>
             <li><a id="iy-share-mail">Email</a></li>
           </ul>
         </div>
       </dialog>
       <dialog class="iy-cite" id="iy-cite" aria-labelledby="iy-cite-title" lang="en">
         <div class="iy-cite-head"><p class="iy-cite-title" id="iy-cite-title">Cite this report</p><form method="dialog"><button class="iy-search-close" aria-label="Close">Esc</button></form></div>
         <div class="iy-cite-block">
           <div class="iy-cite-label"><span>BibTeX</span><span><a href="IY-TR-2026-01.bib" download>.bib</a><button type="button" class="iy-cite-copy" data-target="iy-cite-bib">Copy</button></span></div>
           <pre id="iy-cite-bib">@techreport{nishihama2026iytr202601,
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
           <div class="iy-cite-label"><span>Text</span><button type="button" class="iy-cite-copy" data-target="iy-cite-text">Copy</button></div>
           <p id="iy-cite-text">Nishihama, D. (2026). <i>NASA CFD verification and validation of IgnisYeet v0.4.0</i> (Technical report IY-TR-2026-01). IgnisYeet. https://www.ddd3h.com/ignisyeet/research/IY-TR-2026-01/</p>
         </div>
       </dialog>
     </div>
   </header>

   <div class="iy-wrap iy-paper-layout">

.. raw:: html
   :file: paper.html

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
       var done = function () { copy.textContent = "Copied"; setTimeout(function () { copy.textContent = "Copy link"; }, 1600); };
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
         var done = function () { btn.textContent = "Copied"; setTimeout(function () { btn.textContent = "Copy"; }, 1600); };
         if (navigator.clipboard && navigator.clipboard.writeText) { navigator.clipboard.writeText(text).then(done); }
       });
     });
   })();
   </script>

   </div>
