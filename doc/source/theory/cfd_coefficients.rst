.. _page-theory-cfd-coefficients:

CFD から係数表を作る式
========================

本章は、SU2 が出した力とモーメントの係数から、飛翔計算が読む係数表を作る式を述べる。
座標と符号の変換、CFD が与えない項の補い方、解いた点から規則的な表の格子への補間、一般表、解いた範囲の外、ロール減衰微係数の順に進む。
解く条件の選び方は :ref:`page-theory-cfd-choose`、方程式は :ref:`page-theory-cfd-equations` にある。表の定義は :ref:`page-theory-aero-tables` と :ref:`page-theory-aero-coefficients` にある。

この章で決めること
--------------------

この章の式そのものは選べないが、次の設定が結果の表に効く。

.. list-table::
   :header-rows: 1
   :widths: 30 40 30

   * - 決めること
     - 設定キー
     - 本章の節
   * - 底面の項を CFD から取るか
     - :ref:`rans_base <cfg-aero-cfd-rans_base>`
     - :ref:`sec-cfd-post`
   * - 迎角と Mach 数の格子、迎角の上限
     - :ref:`machs <cfg-aero-cfd-machs>`、:ref:`alphas_deg <cfg-aero-cfd-alphas_deg>`、:ref:`alpha_max_deg <cfg-aero-alpha_max_deg>`
     - :ref:`sec-cfd-table-build`、:ref:`sec-cfd-region-outside`
   * - 一般表の Mach 数の刻み
     - :ref:`general_mach_step <cfg-aero-cfd-general_mach_step>`
     - :ref:`sec-cfd-general`
   * - ロール減衰を CFD から取るか
     - :ref:`roll_damping <cfg-aero-cfd-roll_damping>`
     - :ref:`sec-cfd-roll-post`

選び方
--------

.. _sec-cfd-supplied:

CFD が与える値とモデルで補う値
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

係数表の各量が、CFD の解から来るか、部品積み上げ法（またはパネル法）のモデルから来るかを :numref:`tbl-cfd-supplied` に示す。
結果を使うときは、モデルで補った量の精度が部品積み上げ法の精度であることに注意する。
:numref:`fig-cfd-supplied` は、その区分と、迎角の軸上での CFD の値とモデルの値の関係を示す。

.. _fig-cfd-supplied:

.. figure:: /_generated/tikz/cfd_supplied.*
   :width: 90%

   CFD の解から得る量とモデルで補う量（上）と、迎角の軸上の区分（下）。見るところは、解いた最大の迎角 :math:`\alpha_e` の先でモデルの値へつなぐことと、モデルで補う量の精度は部品積み上げ法の精度になることである。

.. _tbl-cfd-supplied:

.. list-table:: 係数表の量と出どころ
   :header-rows: 1
   :widths: 24 38 38

   * - 量
     - Euler
     - RANS
   * - 法線力 :math:`C_N`、圧力中心 :math:`x_{cp}`
     - CFD
     - CFD
   * - 軸力の圧力抗力（造波）
     - CFD
     - CFD
   * - 摩擦抗力
     - モデル
     - CFD（粗さの増分はモデル）
   * - 底面抗力
     - モデル（フェアリングの面は力に入れない）
     - ``rans_base`` で、モデル（既定）か CFD
   * - フィン縁、ボートテールの抗力
     - ``panel_mesh`` はモデル。``stl`` と ``step`` は縁の面が CFD の壁面にあるので CFD
     - ``panel_mesh`` は設定の縁の増分をモデルで足す。``stl`` は使えない。``step`` は CFD
   * - 横力 :math:`C_Y`、ヨーモーメント :math:`C_n`
     - 横滑りを解けば CFD
     - 横滑りを解けば CFD
   * - ロール減衰 :math:`C_{lp}`
     - ``roll_damping`` なら CFD、なければモデル
     - モデル
   * - ピッチ・ヨー減衰 :math:`C_{mq}`、:math:`C_{nr}`
     - 傾きと重心は CFD、ばらつきはモデル
     - 同左
   * - ロールのカント強制 :math:`C_{l\delta}`
     - モデル
     - モデル
   * - 解いた迎角と横滑り角の外側
     - モデル
     - モデル
   * - ロールとピッチ・ヨーの交差する微係数
     - どの手法でもモデルにない
     - 同左

モデル
--------

.. _sec-cfd-post:

SU2 の係数から機体軸の係数へ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

機体はメッシュに固定されているので、SU2 の力の係数 :math:`C_{Fx},C_{Fz}` はそのまま機体軸の係数である。

.. math::
   :label: eq-cfd-body

   C_N=C_{Fz},\qquad C_A^{\mathrm{CFD}}=C_{Fx},\qquad m=C_N\,x_{cp}=-C_{My}\,d

SU2 はモーメントを :math:`\bm{M}=\bm{r}\times\bm{F}` で計算するので、その :math:`y` 成分は :math:`zF_x-xF_z` である。
ノーズ先端から後方 :math:`x_{cp}` に働く正の法線力は負の :math:`C_{My}` を与える。圧力中心は

.. math::
   :label: eq-cfd-xcp

   x_{cp}=\frac{m}{C_N}=-\frac{C_{My}\,d}{C_N}

である。:math:`|C_N|` が小さいときは定義しない。:math:`C_L,C_D` しか得られないときは、自由流の方向で回転して :math:`C_{Fx}=C_D\cos\alpha-C_L\sin\alpha`、:math:`C_{Fz}=C_D\sin\alpha+C_L\cos\alpha` とする。

.. _sec-cfd-body6:

6 成分の係数
~~~~~~~~~~~~

横滑りやロールを解くときは、SU2 の 6 成分の力とモーメントの係数 :math:`C_{Fx},C_{Fy},C_{Fz},C_{Mx},C_{My},C_{Mz}` を使う。
CFD の座標は機体座標を :math:`y` まわりに 180 度回したもの（:math:`\mathrm{diag}(-1,1,-1)`）で、モーメントは軸性ベクトルなので、機体軸の係数（:eq:`eq-coeff-6`）は

.. math::
   :label: eq-cfd-body6

   C_X=-C_{Fx},\quad C_Y=C_{Fy},\quad C_Z=-C_{Fz},\qquad
   C_l=-C_{Mx},\quad C_m=C_{My},\quad C_n=-C_{Mz}

となる。力は :math:`qS`、モーメントはノーズ先端まわりで :math:`qSd` で無次元化している。
:eq:`eq-cfd-body` との関係は、:math:`C_Z=-C_N`、:math:`C_m=x_{cp}C_Z/d` である（:eq:`eq-coeff-6-axisym`）。
半領域の解では、:math:`y=0` の対称面が半分の機体の圧力を受けるので、:math:`C_Y,C_l,C_n` は対称面による見かけの値である。機体は :math:`y=0` について対称なので、半領域のケースではこれらを 0 にする。
鏡映の像は、:math:`z\to-z` で :math:`(\alpha,\beta)\to(-\alpha,\beta)` となり :math:`C_Z,C_l,C_m` の符号が変わる。
:math:`y\to-y` では :math:`(\alpha,\beta)\to(\alpha,-\beta)` となり :math:`C_Y,C_l,C_n` の符号が変わる。

軸力
~~~~

軸力 :math:`C_A` は、CFD が供給しない項を部品積み上げ法で補う。
:math:`\alpha`・Mach 数ごとの CFD の軸力 :math:`C_A^{\mathrm{CFD}}` に、:eq:`eq-cd0` の成分のうち CFD が与えない項を、迎角の補正 :math:`f(\alpha)`\ （:eq:`eq-ca-alpha`）をかけて足す。

* **Euler**\ ：CFD は圧力による抗力（造波抗力）だけを与える。底面は力の積分から外してあるので、

  .. math::
     :label: eq-cfd-ca

     C_A=\max\Bigl(C_A^{\mathrm{CFD}}+f(\alpha)\bigl(C_{D,f}+C_{D,\mathrm{base}}+C_{D,\mathrm{fin}}+C_{D,\mathrm{bt}}+C_{D,\mathrm{extra}}\bigr),\,0\Bigr)

  である。足すのは摩擦、底面、フィン縁（丸い前縁と四角い後縁）、ボートテール、追加抗力で、ノーズの圧力抗力 :math:`C_{D,\mathrm{nose}}` は **含めない**\ （CFD が与えるため）。
  ``surface = "stl"`` と ``"step"`` では、フィンの前縁と後縁の面が CFD の壁面にあり、その圧力抗力は :math:`C_A^{\mathrm{CFD}}` にすでに入っているので、フィン縁の項 :math:`C_{D,\mathrm{fin}}` を足さない（足すと二重に数える）。
  ``panel_mesh`` の形状は、抽出した厚みのフィンを作り直すので、この項を足す。
  ``"stl"`` と ``"step"`` で、フィンの後縁が底面にそろっている（根元と翼端の後縁がどちらも底面の平面 :math:`x=L` から :math:`10^{-4}L` 以内にある）と、後縁の面は ``base`` の境界に入って力に積分されない。その底面抗力を次の項で足す。

  .. math::
     :label: eq-cfd-te-flush

     C_{D,\mathrm{TE,flush}}=\frac{C_{D,\mathrm{base}}(M)\,n\,b\,t}{S_{\mathrm{ref}}}

  ここで :math:`C_{D,\mathrm{base}}(M)` は部品積み上げ法の底面抗力係数、:math:`n` はフィンの枚数、:math:`b` はスパン、:math:`t` は抽出した平均のフィンの厚さである。
  この項は :eq:`eq-cfd-ca` の和に入れ、``fin_te_square = false`` のとき、後縁が底面から内側にあるとき、``panel_mesh`` のときは 0 である。
  平均の厚さを使うので、後縁が鈍いくさび形のとき、実際の底面抗力より小さい。燃焼中の値では、底面の項に、ノズル出口の面積を除いた底面積を使う。

* **RANS**\ ：壁面の軸力 :math:`C_{Fx}(\mathrm{wall})` は、圧力と摩擦、フィンの縁とボートテールの力を含む。底面の項は ``rans_base`` で選び、

  .. math::
     :label: eq-cfd-ca-rans

     C_A=\max\Bigl(C_{Fx}(\mathrm{wall})+C_{A,\mathrm{base}}+f(\alpha)\bigl(C_{D,\mathrm{extra}}+\Delta C_{D,\mathrm{rough}}+\Delta C_{D,\mathrm{fin}}\bigr),\,0\Bigr),\qquad
     C_{A,\mathrm{base}}=\begin{cases}f(\alpha)\,C_{D,\mathrm{base}} & (\text{rans\_base = model})\\ C_{Fx}(\mathrm{base}) & (\text{rans\_base = cfd})\end{cases}

  とする。``"model"``\ （既定）は部品積み上げ法の底面抗力（燃焼中はノズル出口の面積を除いた底面積）、``"cfd"`` は RANS が平らな底面に積分した力である。
  ``"model"`` で後縁がそろっていれば、:eq:`eq-cfd-te-flush` を :math:`f(\alpha)` をかけて足す。``"cfd"`` では足さない（後縁の面は ``base`` の力に入っている）。
  ``"cfd"`` の燃焼中の値は、ノズル出口の面積にかかる底面圧力の寄与を引く。
  RANS の壁は滑らかなので、設定の粗さの効果を、部品積み上げ法の粗い壁と滑らかな壁の摩擦の差として足す。:math:`C_{D,f}` は粗さ :math:`k_s` の摩擦（:eq:`eq-cd0`）、:math:`c_f` は摩擦係数の式で、同じ Reynolds 数での比を使い、

  .. math::
     :label: eq-cfd-rans-rough

     \Delta C_{D,\mathrm{rough}}=C_{D,f}\Bigl(1-\frac{c_f(Re,M,0)}{c_f(Re,M,k_s/L)}\Bigr)

  とする（粗さ 0 なら 0）。
  作り直した形状（``panel_mesh``）では、フィンの前縁と後縁を弦長の割合 :math:`e=0.02` で鈍くしてあり（:ref:`sec-cfd-rebuild`）、その縁の面の厚さは最大の厚さの :math:`4e(1-e)=7.8` % である。
  設定の縁（``fin_le``、``fin_te``）の抗力は縁の厚さに比例するとして、部品積み上げ法のフィン縁の項 :math:`C_{D,\mathrm{fin}}` から、鈍くした縁がすでに壁面の力に持つ分を引いて

  .. math::
     :label: eq-cfd-rans-fin

     \Delta C_{D,\mathrm{fin}}=C_{D,\mathrm{fin}}\bigl(1-4e(1-e)\bigr)

  を足す。``surface = "step"`` では縁の面が CFD の壁面にあるので :math:`\Delta C_{D,\mathrm{fin}}=0` である。
  どちらの増分もモデルによる補正で、CFD の解ではない。部品積み上げ法の摩擦とボートテールの項は足さない。
  ケースごとの壁面と底面の軸力、部品積み上げ法の値、増分は ``cfd_report.json`` の ``axial_breakdown`` に並べて記録する。

亜音速の Euler では、CFD の軸力の理論値は 0（d'Alembert のパラドックス）である。実際の値は小さな負の数（サンプル機体の迎角 0〜8° で :math:`-0.01` から :math:`-0.05`）になり、そのまま合計に入る。

ピッチ減衰
~~~~~~~~~~

定常な CFD はピッチ減衰を与えない。そこで :ref:`sec-damping` の和 :math:`S_0,S_1,S_2`\ （:ref:`page-theory-barrowman` の :eq:`eq-body-sums` と同じ定義）は、
部品積み上げ法（パネル法があればその値）の **ばらつき** を残したまま、全体の傾きと重心を CFD の値に取り替える。
モデルの和を :math:`S_k^{m}` として、そのまわりのばらつき（分散）

.. math::
   :label: eq-cfd-var

   \sigma^2=\frac{S_2^m}{S_0^m}-\left(\frac{S_1^m}{S_0^m}\right)^2

を保ち、CFD の :math:`C_{N\alpha}` と小迎角の圧力中心 :math:`x_{cp0}` から

.. math::
   :label: eq-cfd-damp

   S_0=C_{N\alpha},\qquad S_1=C_{N\alpha}\,x_{cp0},\qquad S_2=C_{N\alpha}\bigl(x_{cp0}^2+\sigma^2\bigr)

とする。これらの和から :math:`C_{mq}`、:math:`C_{nr}` が決まる関係は :ref:`sec-aero-damping` にある。CFD が直接与えるのは傾きと重心位置だけである。

.. _sec-cfd-table-build:

係数表の作成（軸対称表）
~~~~~~~~~~~~~~~~~~~~~~~~

解いた点は不規則な格子（解いた Mach 数と迎角）なので、それを係数表の規則的な格子に埋める。

1. 各 Mach 数の列で、:math:`C_N` とノーズ周りのモーメント :math:`m` は迎角について奇関数とし、原点 :math:`(0,0)` を通る。
   :math:`C_N,m` と CFD の軸力を、迎角について単調な 3 次補間（PCHIP :cite:`fritsch`）する。
   解いた最大の迎角 :math:`\alpha_e` より大きい表の行は、部品積み上げ法へつなぐ（下記）。端の傾きは延長しない。
2. :math:`C_{N\alpha}` と :math:`x_{cp}(\alpha\to0)` は、最小の二つの正の迎角を通る奇関数の 3 次式 :math:`c_1\alpha+c_3\alpha^3` から求める（当てはめた傾き :math:`c_1` が最小の迎角での割線の傾きから 5 割を超えてずれるときは、割線の傾きを使う）。
   :math:`x_{cp0}` は、モーメント :math:`m` に同じ当てはめをした傾きを :math:`C_{N\alpha}` で割った値である。
   手順 1 の :math:`C_N` と :math:`m` の補間は、原点での傾きをこの当てはめの傾きにして始める。
   こうすると、表の :math:`C_{N\alpha}`、迎角 0 の圧力中心 :math:`x_{cp0}`、減衰の和（:eq:`eq-cfd-damp`）と、表の :math:`C_N` と :math:`m` の行が、一つの曲線から求めたものになる。
3. Mach 数の方向にも、各量を係数表の Mach 数の格子に PCHIP で補間する。解いた Mach 数の範囲の外には外挿せず、最小または最大の Mach 数の値で一定とする。
   迎角 0 の圧力中心は、モーメントの傾き :math:`C_{N\alpha}x_{cp0}` と :math:`C_{N\alpha}` をそれぞれ補間した比とする（:math:`|C_{N\alpha}|\le10^{-9}` では :math:`x_{cp0}` を補間する）。
   0 でない迎角の行が、補間したモーメントを補間した :math:`C_N` で割った圧力中心を持つ（:eq:`eq-xcp-interp`）のと同じ扱いで、解いた Mach 数の間でも、迎角 0 の圧力中心と減衰の和が小さい迎角の行と食い違わない。
   途中の Mach 数の列が失敗していれば、両隣の列で補間して埋める。最小または最大の Mach 数の列の失敗は補えず、エラーになる。
4. 軸力に上の項を足し、ピッチ減衰の和を取り替える。``roll_damping`` が真のときは、:math:`C_{lp}` も CFD の値（:ref:`sec-cfd-roll-post`）にする。

解いた迎角より大きい範囲には CFD のデータがない。端の傾きを延長した直線は、最後の区間で圧力中心が動く向きに圧力中心を際限なく動かし、大きな迎角で重心より前に出すことがある。
そうなると大きな迎角に見かけの安定な釣り合い（トリム）ができ、頂点のあとの弾道落下が、遅く遠くまで流される落下になる。
そこで、Mach 数の列ごとに、:math:`\alpha_e` での CFD の値（:math:`C_{N,e}`、:math:`x_{cp,e}=m_e/C_{N,e}`、:math:`C_{A,e}`）から部品積み上げ法の値（添字 B）へ移す。
部品積み上げ法のフィンの法線力は大迎角で有界で（:ref:`sec-fin-large-alpha`）、フィンの圧力中心は平面形の図心へ動く。

.. math::
   :label: eq-cfd-beyond

   C_N(\alpha) &= C_{N,B}(\alpha)\,\bigl(1+(r-1)\,w(\alpha)\bigr),\qquad r=\frac{C_{N,e}}{C_{N,B}(\alpha_e)}\\
   x_{cp}(\alpha) &= x_{cp,B}(\alpha)+\bigl(x_{cp,e}-x_{cp,B}(\alpha_e)\bigr)\,w(\alpha)\\
   C_A(\alpha) &= C_{A,e}\,\frac{f(\alpha)}{f(\alpha_e)}

ここで :math:`f` は部品積み上げ法の軸力の迎角の倍率（:eq:`eq-ca-alpha`）、:math:`w` は :math:`\alpha_e` で 1、:math:`\alpha_r=\min(2\alpha_e,\,90^\circ)` で 0 になる、両端の傾きが 0 の 3 次 Hermite 補間で、:math:`\alpha\ge\alpha_r` では 0 である。
:math:`r` が正でないとき（:math:`C_{N,e}` と :math:`C_{N,B}(\alpha_e)` の符号が違う）は、比の代わりに差を使い :math:`C_N=C_{N,B}(\alpha)+(C_{N,e}-C_{N,B}(\alpha_e))\,w(\alpha)` とする。
:math:`x_{cp}` は機体の内側（0 から全長）に制限する。
値は :math:`\alpha_e` で CFD の値に一致し、:math:`\alpha_r` から先は部品積み上げ法の法線力と圧力中心になる（軸力は CFD の水準に部品積み上げ法の形を掛けたもので、:math:`90^\circ` で 0）。
このつなぎは ``extrapolation`` の設定によらない。``extrapolation`` は表に記録され、表の格子の外の引き出しだけに効く（:ref:`sec-aero-interp`）。
``examples/sample_cfd.toml``\ （解く迎角 0、4、8°、``alpha_max_deg = 90``）では、8° から 16° の間でつなぎ、16° から 90° は部品積み上げ法の値になる。

.. _sec-cfd-general:

一般表の作成
~~~~~~~~~~~~

一般表（6 成分、:ref:`sec-aero-coeff6`）になる条件は :ref:`sec-cfd-table-kind` にある。
格子は、迎角が :math:`-180^\circ` から :math:`180^\circ`、横滑り角が :math:`-90^\circ` から :math:`90^\circ` である。
どちらも非一様で、:math:`\pm30^\circ` までは 2 度刻み、その外側は 10 度刻み（迎角 61 点、横滑り角 43 点）とする。
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、範囲は ``[aero]`` の ``mach_min`` から ``mach_max`` である。
1 つの Mach 数あたり :math:`61\times43` 点で、Mach 数の刻みを粗くするとファイルが小さくなる（ファイルの形式は :ref:`sec-file-general`）。

Mach 数ごとの作り方は次のとおりである。

1. **対称性による補完**\ 。解いた点に、形状の鏡映（:ref:`sec-cfd-body6`）の像を加える。``alpha_mode = "mirror"`` では、同じ :math:`(\alpha,\beta)` に重なるすべての像を平均し、数値的な非対称を消す。
   そうでなければ、解いた点が像より優先される。``"mirror"`` と ``"offset"`` では、:math:`(0,0)` の点の :math:`C_Y,C_Z,C_m,C_n` を、先に全点から引く。
   :math:`C_l` は鏡映対称な形状でだけ引き、鏡映対称でない形状ではカントしたフィンのロールの強制力として残す。
   像のない成分（鏡映対称でない形状のすべて、``"pitch_plane"`` の :math:`C_Z,C_m`）は、この引き算で取り除かれる（:ref:`sec-cfd-val-asym-cause`）。
   そのあとで、形状の対称性から厳密に 0 になる成分を 0 にする。:math:`y=0` について鏡映対称な形状では、:math:`\beta=0` の点の :math:`C_Y,C_l,C_n` を 0 にし、:math:`z=0` についても対称な形状（``"axisymmetric"``）では、さらに :math:`\alpha=0` の点の :math:`C_Z,C_l,C_m` を 0 にする。
   ``"none"`` では解の値をそのまま使う。
2. **CFD の補間**\ 。解いた範囲 :math:`R=[\alpha_{\min},\alpha_{\max}]\times[\beta_{\min},\beta_{\max}]` の中を、各横滑り角で迎角について PCHIP :cite:`fritsch` し、その値を横滑り角について PCHIP する（変数分離）。
3. **モデルで埋める**\ 。:math:`R` の外側は、部品積み上げ法の軸対称表を 6 成分に展開した値 :math:`T`\ （:eq:`eq-coeff-6-axisym`、全迎角 90 度まで）の変化を、:math:`R` の境界の CFD の値に合わせて足す。
   :math:`R` の内側に最も近い点を :math:`(\alpha_c,\beta_c)` として、

   .. math::
      :label: eq-cfd-fill

      C(\alpha,\beta)=F(\alpha_c,\beta_c)+T(\alpha,\beta)-T(\alpha_c,\beta_c)

   とする。境界をまたいで連続である。:math:`R` の内側では :math:`C=F` である。
4. **軸力**\ 。CFD の :math:`C_A^{\mathrm{CFD}}=-F_{C_X}` に、:eq:`eq-cfd-ca` と同じ項を足す。迎角の補正は :math:`(\alpha_c,\beta_c)` の全迎角 :math:`\cos\alpha_t=\cos\alpha\cos\beta` から :math:`\alpha_t'=\min(\alpha_t,\pi-\alpha_t)` で決める。燃焼中と燃焼後の :math:`C_X` が得られる。
5. **Mach 数の方向**\ 。格子の点ごとに、解いた Mach 数の値を PCHIP で補間する。解いた範囲の外は外挿せず、端の値で一定とする（``extrapolation`` にかかわらない）。最小または最大の Mach 数のケースが全部失敗していれば、補えないのでエラーにする。

ピッチとヨーの減衰の和は、零角度の傾きから別々に取り替える。
ピッチは :math:`\beta=0` で、ヨーは :math:`\alpha=0` で、奇関数の 3 次式 :math:`c_1\theta+c_3\theta^3` を最小の二つの正の角度に当てはめて傾きを求め、

.. math::
   :label: eq-cfd-slopes

   \begin{aligned}
   C_{N\alpha}^{q}&=-\frac{\partial C_Z}{\partial\alpha},&\quad
   x_{cp0}^{q}&=d\,\frac{\partial C_m/\partial\alpha}{\partial C_Z/\partial\alpha},\\
   C_{N\alpha}^{r}&=-\frac{\partial C_Y}{\partial\beta},&\quad
   x_{cp0}^{r}&=-d\,\frac{\partial C_n/\partial\beta}{\partial C_Y/\partial\beta}
   \end{aligned}

とする。:eq:`eq-cfd-damp` をピッチ（:math:`S^q`）とヨー（:math:`S^r`）にそれぞれ適用する。
横滑りを解かないときは、ヨーの和をピッチの和に等しくし、``cfd_report.json`` の ``yaw_damping_from_pitch`` が真になる。ピッチとヨーを別々に求めるので、機体が非対称なときは異なる値になる。

ロールのカント強制 :math:`C_{l\delta}` は、部品積み上げ法の値を使う。
一般表は、表の :math:`C_l` に含まれるカント :math:`\delta_{\mathrm{ref}}` を記録する（:eq:`eq-cl-total`）。
STL モードでは、形状（STL）にカントが含まれているので、:math:`\delta_{\mathrm{ref}}` は設定したフィンのカントである。
カントの影響は :math:`C_l` にすでに入っており、:math:`\delta=\delta_{\mathrm{ref}}` の飛翔では強制項を足さない（二重に数えない）。
``panel_mesh`` は形状にカントがないので、:math:`\delta_{\mathrm{ref}}=0` である。

.. _sec-cfd-region-outside:

CFD の領域の外
~~~~~~~~~~~~~~

一般表の値のうち、解いた迎角と横滑り角の範囲の外側は、CFD の結果ではなくモデルの値である。
解いた範囲 :math:`R` の内側は CFD の補間値で、外側は :eq:`eq-cfd-fill` により境界をまたいで連続につないだモデルの値である。
展開は軸対称な形状のものなので、非対称な形状の :math:`R` の外側では当てにならない。
どこまでが CFD の値かは、``cfd_report.json`` の ``cfd_region``\ （迎角と横滑り角の最小・最大）と ``model_outside_cfd_region`` で確かめる。係数表のファイルには領域の情報は付かない。

Mach 数の方向は異なり、解いた Mach 数の範囲の外は、``extrapolation`` にかかわらず、端の値で一定とする。
表の Mach 数の格子が解いた範囲より広いと、``cfd_report.json`` に ``mach_clamped_above``\ （それより上で一定になる Mach 数）または ``mach_clamped_below`` を記録し、同じ内容の警告を ``warnings`` に加える。
軸対称表では、解いた最大の迎角より大きい行を部品積み上げ法へのつなぎ（:eq:`eq-cfd-beyond`）で埋める。
CFD が与えない項とピッチ・ヨー減衰の和は、部品積み上げ法（またはパネル法）のモデルから補う（:ref:`sec-cfd-post`）。

.. _sec-cfd-roll-post:

ロール減衰微係数
~~~~~~~~~~~~~~~~

回転座標系の 2 ケース（:ref:`sec-cfd-roll-cases`）から、Mach 数ごとに

.. math::
   :label: eq-cfd-clp

   C_{lp}=\frac{C_l(+\hat p)-C_l(-\hat p)}{2\,\hat p}

を求める（:math:`C_l=-C_{Mx}`）。差を取るので、カントや非対称による零回転のずれは消える。
次の場合、その Mach 数の値は使わず、部品積み上げ法の :math:`C_{lp}` に替えて、理由を警告と ``cfd_report.json`` の ``clp`` に記録する。

* 2 ケースのどちらかが失敗したか、値が有限でない。
* フィンがあるのに :math:`C_{lp}` が正である（ロール減衰は負でなければならない）。
* フィンがあるのに :math:`|C_{lp}|<10^{-6}` である。

表の値は、部品積み上げ法の :math:`C_{lp}(M)` に、使える Mach 数での CFD / 部品積み上げ法の比を PCHIP で補間して（範囲外は端の値で一定）かけた値である（部品積み上げ法の値が 0 に近いときは、CFD の値そのものを補間する）。
使える Mach 数がなければ部品積み上げ法の値になる。``clp_source`` は ``"cfd"``\ （すべての Mach 数が CFD）、``"barrowman"``\ （解かない、またはすべて不採用）、``"mixed"`` のいずれかである。
ロール減衰は Euler の解であり、粘性の寄与（フィンの摩擦）は含まない。

.. _sec-cfd-flow:

流れ場を見る
--------------

係数表の値は、流れ場の積分である。流れ場を見ると、法線力、圧力中心、軸力が壁面のどこから生まれるかがわかる。
サンプル機体の :math:`\alpha=4^\circ`、Euler、``wall_size`` 2 mm（フィンの面 1 mm、142 万セル）、一様流からの開始で解いたときの図を示す。
データは ``doc/data/cfd/fields/`` に保存してある。

:numref:`fig-cfd-mach-field` は対称面 :math:`y=0` での Mach 数の分布である。

* :math:`M_\infty=0.8`\ （上）: ノーズ先端によどみ点（青）があり、ノーズの肩で流れが加速する（橙）。
* :math:`M_\infty=2.0`\ （下）: ノーズ先端から斜めに広がる青い帯がノーズの衝撃波で、迎角があるので風上側（:math:`z<0`）のほうが強い。フィンの前縁からも衝撃波が出ている。

.. _fig-cfd-mach-field:

.. figure:: /_generated/plots/cfd_mach_field.*
   :width: 100%

   対称面 :math:`y=0` での Mach 数の分布（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。
   流れは左から右へ向かう。色の中心は一様流の Mach 数で、青が一様流より遅い領域、橙が速い領域を表す。範囲を超える値は端の色で、灰色は機体の内部である。

:numref:`fig-cfd-cp-surface` は壁面の圧力係数 :math:`C_p` の分布で、半モデルの解を :math:`y=0` 面で鏡映して全機を描いた。機体の下側（風上側）から見ている。

* ノーズでは風上側の :math:`C_p` が風下側より高く、この差が法線力を生む。
  先端から 0.02 から 0.25 m の範囲で、風上側（:math:`z<0`）と風下側（:math:`z>0`）の半分ごとに面積で重み付けした平均は、:math:`M_\infty=0.8` で風上側 :math:`-0.015`、風下側 :math:`-0.069`、:math:`M_\infty=2.0` で風上側 0.104、風下側 0.045 である。
* :math:`M_\infty=0.8` では、尾部フェアリングの付け根（底面の縁）で流れが曲がって加速し、環状の負圧ができる（最小で :math:`C_p\approx-0.9`）。

.. _fig-cfd-cp-surface:

.. figure:: /_generated/plots/cfd_cp_surface.*
   :width: 100%

   壁面の圧力係数（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。左はノーズ、右はフィンと尾部フェアリング。ノーズの風上側と風下側の色の差と、フィンの縁の分布を見る。色の範囲は図ごとに異なる。

これらの図は、係数表の値が壁面の圧力分布を積分した結果であることを示す。法線力は風上側と風下側の圧力差、軸力の圧力抗力はノーズとフィンの前縁の圧力、底面の負圧は Euler では底面抗力のモデルで補う部分に当たる。

適用範囲と限界
----------------

* 係数表は準定常である。ピッチ・ヨー減衰は傾きと重心だけが CFD で、ばらつきはモデルである。ロールとピッチ・ヨーの交差する微係数は扱わない。
* 解いた迎角と横滑り角の外側はモデルの値である。軸対称表は、解いた最大の迎角の 2 倍（上限 90°）から先が部品積み上げ法の値になる。
* Mach 数は外挿せず、端の値で一定とする。
* Euler の底面、摩擦、フィン縁の抗力は経験式による。``stl`` と ``step`` のフィン縁は、縁の面を ``fin_size`` で解像した場合に限り CFD の値になる（:ref:`sec-cfd-mesh-size`）。
* フィンの後縁が鈍いくさび形のとき、そろった後縁の底面抗力は過小になる。
* ロール減衰は Euler の解で、フィンの摩擦を含まない。符号とほぼ 0 の検査に通らない Mach 数は部品積み上げ法の値になる。

精度の証拠
------------

* **符号と一貫性（検証）**\ ：ロール減衰の符号と無次元化、6 成分への写像、横滑りの符号は、軸対称表を展開した値と一致する（:ref:`sec-cfd-val-roll`）。表の :math:`C_{N\alpha}` は法線力の補間曲線の原点での傾きと 0.1 % 以内で一致する（:ref:`sec-vaero-cfd`）。
* **非対称性の除去（検証）**\ ：``alpha_mode = "mirror"`` と :math:`z` 対称メッシュで :math:`\alpha=0` の見かけの非対称性が消える（:ref:`sec-cfd-val-asym`）。
* **手法どうし（相互比較）**\ ：CFD と部品積み上げ法、パネル法の係数の比較は :ref:`sec-vaero-compare` にある。
* **実測との比較（妥当性確認）**\ ：Basic Finner の超音速の射場試験との比較だけである（:ref:`page-vv-validation-basic-finner`）。

関連する設定キー
------------------

値の完全な定義は :ref:`page-reference-config-cfd` にある。

.. list-table::
   :header-rows: 1
   :widths: 24 22 54

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`rans_base <cfg-aero-cfd-rans_base>`
     - ``"model"``
     - ``"cfd"`` は底面の力を CFD から取る（検証していない）
   * - :ref:`alpha_mode <cfg-aero-cfd-alpha_mode>`
     - ``"mirror"``
     - 一般表の :math:`\alpha=\beta=0` の引き算と像の平均の扱いが変わる
   * - :ref:`general_mach_step <cfg-aero-cfd-general_mach_step>`
     - 0.1
     - 粗くすると一般表のファイルが小さくなる
   * - :ref:`roll_damping <cfg-aero-cfd-roll_damping>`
     - ``false``
     - ``true`` で :math:`C_{lp}` を CFD から取る
   * - :ref:`extrapolation <cfg-aero-extrapolation>`
     - ``[aero]`` の値
     - 表の格子の外の引き出しだけに効く。CFD の領域の外のつなぎには効かない
