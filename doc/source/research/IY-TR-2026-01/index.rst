:orphan:

.. meta::
   :description: NASA WIND/NPARC の検証と妥当性確認の手順を IgnisYeet の空力計算に適用し、空力係数に数値不確かさの帯を付ける研究の報告。

IY-TR-2026-01：NASA WIND の V&V 手順による IgnisYeet の空力計算の検証と妥当性確認
==========================================================================================

.. raw:: html

   <div class="iy">

   <header class="iy-hero iy-page-hero">
     <div class="iy-wrap">
       <p class="iy-crumb"><a href="../">R&amp;D</a></p>
       <p class="iy-badges"><span class="iy-badge">IY-TR-2026-01</span><span class="iy-badge">技術報告</span><span class="iy-badge iy-badge-status">実施中</span></p>
       <p class="iy-page-title iy-report-title" role="presentation">NASA WIND の V&amp;V 手順による IgnisYeet の空力計算の検証と妥当性確認</p>
       <p class="iy-meta">西濱大将 (NISHIHAMA Daisuke) ／ 2026 年</p>
     </div>
   </header>

   <div class="iy-wrap iy-report">

   <section id="abstract">
     <h2>概要</h2>
     <p>NASA Glenn の WIND/NPARC チュートリアルが定める検証と妥当性確認（V&amp;V）の手順を、IgnisYeet の空力計算に適用する。目的は、IgnisYeet が出す空力係数に数値不確かさの帯を付け、その帯を実測の誤差と並べて示すことである。帯のない比較では、計算と実測のどちらが正しいかを決められない。</p>
     <p>本報告は段階ごとに結果を追記する。現在は計画が定まった段階であり、数値の結果はまだ載せていない。</p>
   </section>

   <section id="terms">
     <h2>用語</h2>
     <p>本報告では次の三つを別の名前で呼び、混ぜない。CFL3D や FUN3D の結果との比較は相互比較であり、妥当性確認ではない。</p>
     <div class="iy-table"><table>
       <thead><tr><th>名前</th><th>問うこと</th><th>比べる相手</th></tr></thead>
       <tbody>
         <tr><td>検証（verification）</td><td>式を正しく解いているか</td><td>厳密解、高精度解</td></tr>
         <tr><td>相互比較（cross-comparison）</td><td>手法やプログラムの差</td><td>別の手法、別のプログラム</td></tr>
         <tr><td>妥当性確認（validation）</td><td>実機に近いか</td><td>実測値</td></tr>
       </tbody>
     </table></div>
   </section>

   <section id="procedure">
     <h2>手順</h2>
     <p>チュートリアルの手順は次の 6 段階からなる。</p>
     <ol class="iy-steps">
       <li><strong>プログラムの点検</strong>：単体試験と既存の検証の記録を引く。</li>
       <li><strong>反復収束</strong>：残差が十分に落ち、係数が平坦になり、反復の打ち切りによる誤差が格子収束の誤差より小さいことを示す。</li>
       <li><strong>一貫性</strong>：迎角 0 で法線力が 0 になる、迎角の符号を変えると法線力が反転する、など物理が要求する関係を確かめる。</li>
       <li><strong>空間（格子）収束</strong>：3 段以上の格子から観測次数と GCI（Grid Convergence Index）を求める。</li>
       <li><strong>時間収束</strong>：CFD は定常計算なので該当しない。代わりに 6 自由度の積分器の刻み幅の収束を示す。</li>
       <li><strong>高精度解・実測との比較</strong>：厳密解との比較を検証、実測との比較を妥当性確認として分けて述べる。</li>
     </ol>
   </section>

   <section id="layers">
     <h2>三層の構成</h2>
     <p>検証の対象を三つの層に分ける。層 0 と層 1 は同じ厳密解（半頂角 10° の円すいの Taylor–Maccoll 解）を使う。両者の差が、メッシュ生成・表面表現・力の積分というパイプライン固有の誤差を表す。</p>
     <div class="iy-table"><table>
       <thead><tr><th>層</th><th>実行するもの</th><th>確かめる対象</th></tr></thead>
       <tbody>
         <tr><td>0</td><td>SU2 を IgnisYeet と同じ数値設定で直接実行</td><td>ソルバと数値設定</td></tr>
         <tr><td>1</td><td>IgnisYeet の CAD → メッシュ → SU2 → 係数の流れ全体</td><td>パイプライン</td></tr>
         <tr><td>2</td><td>IgnisYeet をサンプル機体と Basic Finner に適用</td><td>実用条件での誤差と実測との差</td></tr>
       </tbody>
     </table></div>
     <p>層 0 では、解析解を持つ次の算例を解く。</p>
     <div class="iy-table"><table>
       <thead><tr><th>算例</th><th>Mach 数</th><th>厳密解</th></tr></thead>
       <tbody>
         <tr><td>15° くさびの斜め衝撃波</td><td>2.5</td><td>θ–β–M 関係式</td></tr>
         <tr><td>半頂角 10° の円すい</td><td>2.35</td><td>Taylor–Maccoll</td></tr>
         <tr><td>15° の Prandtl–Meyer 膨張</td><td>2.5</td><td>Prandtl–Meyer 関数</td></tr>
         <tr><td>層流平板</td><td>0.2</td><td>Blasius 解</td></tr>
         <tr><td>乱流平板</td><td>0.2</td><td>Cf の相関式、NASA TMR の参照解</td></tr>
       </tbody>
     </table></div>
     <p>層 2 の格子収束は、サンプル機体（Mach 0.3、0.8、2.0）と Basic Finner（Mach 2.375 の Euler、Mach 1.832 の RANS-SA）で調べる。対象の量は CN<sub>α</sub>、CA、圧力中心の位置である。Basic Finner は DREV-TM-9703 の実測と比べ、実測の確率誤差の棒と CFD の GCI の帯を同じ図に描く。</p>
   </section>

   <section id="progress">
     <h2>進み具合</h2>
     <div class="iy-table"><table>
       <thead><tr><th>段階</th><th>内容</th><th>状態</th></tr></thead>
       <tbody>
         <tr><td>A</td><td>GCI の計算と厳密解の実装（NASA の例題で受け入れ試験）</td><td>未着手</td></tr>
         <tr><td>B</td><td>層 0 の 5 算例</td><td>未着手</td></tr>
         <tr><td>C</td><td>層 1 の円すい-円筒</td><td>未着手</td></tr>
         <tr><td>D</td><td>層 2 の点検、反復収束、一貫性</td><td>未着手</td></tr>
         <tr><td>E</td><td>層 2 の格子収束</td><td>未着手</td></tr>
         <tr><td>F</td><td>層 2 の時間収束、モデル不確かさ、妥当性確認</td><td>未着手</td></tr>
         <tr><td>G</td><td>報告の完成と論文の下書き</td><td>未着手</td></tr>
       </tbody>
     </table></div>
   </section>

   <section id="scope">
     <h2>適用範囲</h2>
     <p>本報告は IgnisYeet の現状の評価であり、ソルバや SU2 を書き換えない。乱流モデルは IgnisYeet が持つ RANS-SA だけを扱う。チュートリアルが求める複数の乱流モデルの比較は満たせないので、適用範囲の限界として扱う。非定常の算例と、Basic Finner 以外の新しい実測データは扱わない。</p>
   </section>

   <section id="references">
     <h2>参考文献</h2>
     <ul class="iy-refs">
       <li>NASA Glenn Research Center, WIND/NPARC Validation Archive, <i>CFD Verification and Validation Tutorial</i>. <a href="https://www.grc.nasa.gov/www/wind/valid/tutorial/tutorial.html" rel="noopener">https://www.grc.nasa.gov/www/wind/valid/tutorial/tutorial.html</a></li>
       <li>AIAA, <i>Guide for the Verification and Validation of Computational Fluid Dynamics Simulations</i>, AIAA G-077-1998.</li>
       <li>Celik, I. B., et al., “Procedure for Estimation and Reporting of Uncertainty Due to Discretization in CFD Applications,” <i>Journal of Fluids Engineering</i>, 130(7), 2008.</li>
     </ul>
   </section>

   </div>

   </div>
