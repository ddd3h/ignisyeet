:orphan:

.. meta::
   :description: IY-TR-2026-01. NASA CFD verification and validation of IgnisYeet v0.4.0: whether the NASA WIND/NPARC procedure can quantify the numerical uncertainty of an automated CAD-to-coefficient CFD pipeline.

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
       <p class="iy-actions"><a class="iy-btn iy-btn-primary" href="IY-TR-2026-01.pdf"><svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M12 11v6M9.5 14.5 12 17l2.5-2.5"/></svg>PDF</a></p>
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
   </script>

   </div>
