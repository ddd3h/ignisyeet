空力係数と座標系
================

本章は、係数表に入っている数値が何を意味するかを決める。
機体に働く空気力は、機体に固定した座標軸の 3 方向の力と、3 軸まわりのモーメントに分けられる。
これを動圧と機体の大きさで割って無次元にしたものが空力係数である。
どの手法で作った表も、この定義に従う。飛翔計算も、同じ定義で表を読む。
本章では、座標軸と力・モーメントの向き、6 個の係数の定義、軸対称な機体での特殊形、法線力と圧力中心、
全迎角と迎角、横滑り角の違い、減衰とロールの微係数を述べる。
表の形式と内挿は :doc:`aerodynamics` で、手法ごとの係数の求め方は :doc:`barrowman`、:doc:`panel`、:doc:`cfd` で述べる。

この章で決めること
------------------

係数の組には、軸対称な機体向けの法線力・軸力・圧力中心の組と、一般の機体向けの機体座標の 6 成分の組の 2 通りがある。
前者は軸対称表、後者は一般表の中身である。両者は一対一に対応しており、対応を :numref:`tbl-aero-coeff-map` に示す。

.. _tbl-aero-coeff-map:

.. list-table:: 軸対称表と一般表の係数の対応
   :header-rows: 1
   :widths: 24 24 52

   * - 軸対称表
     - 一般表
     - 関係
   * - 法線力 :math:`C_N`
     - :math:`C_Y,\ C_Z`
     - :math:`C_N` は機軸に垂直な力の大きさ。:math:`(C_Y,C_Z)` はその力を機体座標の成分で表したもの（:ref:`sec-aero-cn-cz`）
   * - 軸力 :math:`C_A`
     - :math:`C_X`
     - :math:`C_X=-C_A` が前向き飛行の関係。符号は抗力の向きで逆になる
   * - 圧力中心 :math:`x_{cp}`
     - :math:`C_m,\ C_n`
     - :math:`x_{cp}` は力の作用点の位置。:math:`C_m,C_n` はノーズ先端まわりのモーメントで、一般表では 1 個の圧力中心では表せない

軸対称表の全迎角 :math:`\alpha_t` は機軸と対気速度のなす角で、一般表の迎角 :math:`\alpha` と横滑り角 :math:`\beta` は、それを機体座標の 2 面に分けたものである（:ref:`sec-aero-alpha-beta`）。
:math:`\beta` は、圧縮性の因子 :math:`\sqrt{|1-M^2|}`\ （部品積み上げ法・パネル法）や衝撃波角（パネル法）にも同じ文字を使うが、本章の :math:`\beta` は横滑り角だけを指す。

機体座標
--------

力とモーメントは、機体に固定した右手系の座標 :math:`(x_b,y_b,z_b)` で表す。
:math:`x_b` は機軸に沿ってノーズ方向、:math:`y_b` と :math:`z_b` はそれに直交する。
原点は重心である。一方、機体上の位置（圧力中心、フィンの位置など）は、ノーズ先端から後方へ測った距離で表す。
座標軸の向きは STL から決まり、その決め方、座標軸の図、角度の定義は :doc:`coordinates` にある（:numref:`fig-body-frame`、:numref:`fig-body-axes`、:ref:`sec-body-basis`）。
ここでは繰り返さない。

.. _sec-aero-coeff-frame:

力とモーメント
--------------

空気力は、機体座標の 3 成分 :math:`\bm{F}=(F_x,F_y,F_z)` で表す。
:math:`F_x` は機軸方向で、前向きが正である。したがって通常の飛行では抗力は :math:`F_x<0` になる。
:math:`F_y,F_z` は機軸に垂直な 2 方向の力である。

空力モーメントは、3 軸まわりの成分 :math:`\bm{M}=(M_x,M_y,M_z)` で表す。
:math:`M_x` はロール、:math:`M_y` はピッチ、:math:`M_z` はヨーのモーメントで、向きは各軸に対して右ねじの向きを正とする。
空力モーメントの基準点は、重心ではなくノーズ先端である。重心は燃焼とともに動くので、
表には重心に依存しない値を保存し、重心まわりのモーメントは飛翔計算で求める。
:numref:`fig-aero-forces6` に、力、モーメント、基準点を示す。

.. _fig-aero-forces6:

.. figure:: _generated/tikz/aero_forces6.*
   :width: 85%

   機体座標の空気力 :math:`F_x,F_y,F_z` とモーメント :math:`M_x,M_y,M_z`。基準点はノーズ先端である。
   側面図で :math:`y_b` 軸は紙面の奥向きに描いている。

.. _sec-aero-coeff6:

無次元係数
----------

機体が軸対称でないときは、法線力と軸力の 2 成分では力とモーメントを表せない。
一般表は、機体座標 :math:`(x_b,y_b,z_b)`\ （:ref:`sec-body-basis`）の空気力 :math:`\bm{F}=(F_x,F_y,F_z)` と、
ノーズ先端まわりの空力モーメント :math:`\bm{M}=(M_x,M_y,M_z)` を、動圧 :math:`q=\tfrac12\rho V^2`、基準面積 :math:`S=\pi d^2/4`、基準直径 :math:`d` で割った 6 個の無次元係数で表す。

.. math::
   :label: eq-coeff-6

   \begin{aligned}
   C_X&=\frac{F_x}{qS}, & C_Y&=\frac{F_y}{qS}, & C_Z&=\frac{F_z}{qS},\\
   C_l&=\frac{M_x}{qSd}, & C_m&=\frac{M_y}{qSd}, & C_n&=\frac{M_z}{qSd}
   \end{aligned}

:math:`C_X,C_Y,C_Z` は機体座標の力の成分そのものである（抗力では :math:`C_X<0`）。
:math:`C_l,C_m,C_n` は :math:`x_b,y_b,z_b` まわりのモーメントで、それぞれロール、ピッチ、ヨーのモーメントである。
表の 6 係数は :math:`(M,\alpha,\beta)` の関数で、6 成分の準定常空力を表す。
角速度による空気力の変化は、この 6 係数には含まれず、減衰の微係数（:ref:`sec-aero-damping`）として別に加える。

.. _sec-aero-moment-ref:

モーメントの基準点と重心への移送
--------------------------------

表のモーメントの基準点は、表の付随ファイルの ``moment_ref``\ （値は ``"nose_tip"``）が示すノーズ先端で、重心には依存しない。
重心まわりのモーメントは、飛翔計算で重心の位置から求める。ノーズ先端から重心へ基準点を移すと、力が作るモーメントが加わるので、
重心の位置が燃焼で変わっても、同じ表をそのまま使える。
その式は :eq:`eq-aero-moment6` で、:doc:`dynamics` にある。ここでは再定義しない。

軸対称な場合の特殊形
--------------------

機体が軸対称なときは、横向きの空気力は迎角の面内にあって対気速度の横成分と逆を向き、軸力は機軸に沿って対気速度に逆らう。
そのため、法線力と軸力の 2 係数と圧力中心から、6 係数が決まる。
対気速度の横成分の向きの単位ベクトルを :math:`(0,e_y,e_z)=(0,u_y,u_z)/u_\perp` として

.. math::
   :label: eq-coeff-6-axisym

   C_X=-C_A\,\sgn(u_x),\qquad (C_Y,C_Z)=-C_N\,(e_y,e_z),\qquad
   C_m=\frac{x_{cp}\,C_Z}{d},\qquad C_n=-\frac{x_{cp}\,C_Y}{d}

である。ここで :math:`\bm{u}=(u_x,u_y,u_z)` は機体座標の対気速度、:math:`u_\perp=\sqrt{u_y^2+u_z^2}` である（:eq:`eq-alpha`）。
:math:`C_l` は、フィンにカントがあるときのロールのモーメントで、:ref:`sec-roll-moment` で扱う。
ロールの微係数として、カントあたりのロールモーメント係数 :math:`C_{l\delta}`\ [1/rad] と、ロール減衰微係数 :math:`C_{lp}` を表に持つ。
軸力 :math:`\sgn(u_x)` の因子は、後ろ向きの飛行（:math:`u_x<0`）でも軸力が対気速度に逆らう向きになるためである。

.. _sec-aero-xcp:

法線力・軸力・圧力中心
----------------------

軸対称な機体では、機体に働く力を機軸方向の軸力と機軸に垂直な法線力に分けると扱いやすい。
迎角が大きくなると法線力が増え、その作用点が圧力中心である。圧力中心が重心より後ろにあれば、機首を気流に向ける復元のモーメントが生じる。
軸対称表は、この法線力、軸力、圧力中心と、迎角 0 での法線力の傾きを持つ（:numref:`fig-body-frame`）。
ここからは、これらを式で定義する。

力を **軸力** :math:`A` と **法線力** :math:`N` とし、:math:`q`、:math:`S` で

.. math::
   :label: eq-coeff-def

   N = C_N\,q\,S,\qquad A = C_A\,q\,S

と無次元化する。:math:`C_N` は迎角とともに増え、:math:`C_A` は迎角 0 での抗力係数 :math:`C_{D0}` に近い。
法線力の作用点を **圧力中心** :math:`x_{cp}` と呼ぶ。
:math:`x_{cp}` が重心 :math:`x_{cg}` より後ろにあれば、迎角を打ち消す向きのモーメントが生じ、機体は静的に安定である。
その余裕を基準直径で割った

.. math::
   :label: eq-static-margin

   \text{静安定余裕} = \frac{x_{cp}-x_{cg}}{d}\quad[\mathrm{cal}]

を **静安定余裕**\ （キャリバー）と呼ぶ。圧力中心と重心の位置関係で安定性を判定するのは、Barrowman :cite:`barrowman` 以来の慣例である。一般に 1〜2 cal 以上が目安とされる。

迎角 0 付近での傾き :math:`C_{N\alpha}=\partial C_N/\partial\alpha|_{\alpha=0}` を **法線力傾斜** と呼ぶ。
各部品の :math:`C_{N\alpha,i}` と圧力中心 :math:`x_i` が分かれば、全体の圧力中心はモーメントのつり合いから

.. math::
   :label: eq-cp-sum

   x_{cp} = \frac{\sum_i C_{N\alpha,i}\,x_i}{\sum_i C_{N\alpha,i}}

で求まる。これが部品積み上げ法の考え方である :cite:`barrowman`。

.. _sec-aero-cn-cz:

:math:`C_N` と :math:`C_Z`、:math:`x_{cp}` と :math:`C_m` の違い
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

:math:`C_N` は機軸に垂直な力の大きさで、負にならない。:math:`C_Z` は機体座標の :math:`z_b` 方向の成分で、符号を持つ。
:eq:`eq-coeff-6-axisym` から :math:`C_N=\sqrt{C_Y^2+C_Z^2}` で、:math:`\alpha>0` で :math:`\beta=0` のとき :math:`C_Z=-C_N` である。
同様に、:math:`x_{cp}` は力の作用点の位置（長さ）で、:math:`C_m,C_n` はノーズ先端まわりのモーメントの係数である。
軸対称な機体では :math:`C_m=x_{cp}C_Z/d`、:math:`C_n=-x_{cp}C_Y/d` が成り立つ。
:math:`C_A` と :math:`C_X` も同様に、:math:`C_A` は大きさ、:math:`C_X` は符号付きの機体座標の成分である。

一般表では、法線力の作用点を 1 つの圧力中心 :math:`x_{cp}` で表す意味が本質的でない。非対称な機体は、横方向の力 :math:`(C_Y,C_Z)` とモーメント :math:`(C_m,C_n)` が独立に決まり、1 つの位置では両方を表せないからである。
飛翔計算は一般表では :math:`x_{cp}` を使わず、6 係数から直接モーメントを求める。
``trajectory.csv`` の ``x_cp``\ （その状態の迎角での圧力中心の表示）には、:eq:`eq-xcp-eff` の **等価な圧力中心** を使う。
静的な安定の判定には、迎角 0 と横滑り角 0 での傾きから、ピッチ面とヨー面の静安定余裕を別々に求めて小さいほうを使う（:ref:`sec-static-margin-general`）。

軸対称表と一般表の関係
----------------------

軸対称表と一般表は、機体が軸対称なときに一対一に対応する（:numref:`fig-aero-tables`）。
軸対称表を一般表に展開するときは、:math:`(\alpha,\beta)` ごとに全迎角を求めて軸対称表を引き、:eq:`eq-coeff-6-axisym` で 6 係数にする。
展開した一般表は、軸対称表と同じ情報しか持たない。
展開の手順、表を引く角度の範囲、非対称な機体の扱いは :ref:`sec-aero-table` に述べる。

.. _sec-aero-alpha-beta:

:math:`\alpha`、:math:`\beta`、:math:`\alpha_t` の違い
------------------------------------------------------

全迎角 :math:`\alpha_t` は、機軸と対気速度のなす角で、0 から 180° の値をとる。向きの情報は持たない。
迎角 :math:`\alpha` は、対気速度の :math:`x_b`–:math:`z_b` 面内の成分から決まる角、横滑り角 :math:`\beta` は :math:`y_b` 方向の成分から決まる角で、どちらも符号を持つ。
これらの関係は

.. math::

   \cos\alpha_t=\cos\alpha\,\cos\beta

である。:math:`\beta=0` ならば :math:`\alpha_t=|\alpha|`、:math:`\alpha=0` ならば :math:`\alpha_t=|\beta|` となる。
定義の式（:eq:`eq-alpha-beta`、:eq:`eq-alpha`）と図（:numref:`fig-body-axes`）は :ref:`sec-aero-angles` にある。

係数表を引くときの角度は、軸対称表では全迎角 :math:`\alpha_t`\ （実際には :math:`\min(\alpha_t,\pi-\alpha_t)`）、一般表では :math:`\alpha` と :math:`\beta` である。
引く範囲、既定の表が持つ範囲、範囲の外の扱い、後ろ向きの飛行の扱いは :ref:`sec-aero-table` と :ref:`sec-aero-interp` にある。

係数表の CSV の列 ``alpha_deg`` は、軸対称表では全迎角 :math:`\alpha_t`、一般表では迎角 :math:`\alpha` を表す。
外部の表を作るときは、この違いに注意する（:ref:`sec-aero-table-format`）。

.. _sec-aero-damping:

減衰とロールの微係数
--------------------

機体が回転すると、回転を妨げる向きの空力モーメントが生じる。ピッチとヨーの減衰、ロールの減衰、およびフィンのカントによるロールの強制が、この節の対象である。
これらは静的な係数と違い、角速度に比例するので、角速度を無次元化した量についての微係数として表に持つ。
どの量がどの手法から来るかは、手法ごとに違う。

**ピッチ・ヨーの減衰**\ は、実装では次の形で飛翔計算に入る。
重心まわりのピッチ（ヨー）の角速度を :math:`\omega` として、減衰のモーメントは :eq:`eq-damping` の和 :math:`S_0,S_1,S_2` を使い

.. math::

   M = -\frac{qS}{V}\,\omega\,K,\qquad K=\max\!\left(0,\ S_2-2x_{cg}S_1+x_{cg}^2S_0\right)

である。:math:`K` は重心位置 :math:`x_{cg}` に依存する。軸対称表では :math:`S_0,S_1,S_2` をピッチ・ヨーの両方に使い、一般表ではピッチ用の和 :math:`S^q` とヨー用の和 :math:`S^r` を別々に持つ（式は :eq:`eq-moment-damp`、:eq:`eq-moment-damp6` にある）。
これを無次元の角速度 :math:`\hat q=\omega d/(2V)` に対するモーメント係数の微係数で表すと、:math:`C_{mq}=\partial C_m/\partial\hat q` として

.. math::

   C_{mq}=-\frac{2K}{d^2}

である（ヨーの :math:`C_{nr}` は :math:`K_r` で同じ形）。これは実装した関係式を無次元で書き換えたもので、重心位置に依存する。
:math:`C_{mq}` と :math:`C_{nr}` は表に直接の列としては持たず、:math:`S_0,S_1,S_2` から作る。

**ロールの微係数**\ は、:math:`C_{lp}=\partial C_l/\partial\hat p`\ （:math:`\hat p=pd/(2V)`、:math:`p` はロール角速度）として表に直接持つ。
カントあたりのロールモーメント係数 :math:`C_{l\delta}` も表の列である。ロールの式は :eq:`eq-cl-total` である（:ref:`sec-roll-moment`）。

**加速度と回転による項**\ は、次の列で表に持つ（式は :ref:`sec-aero-unsteady`）。

.. list-table::
   :header-rows: 1
   :widths: 16 14 70

   * - 列
     - 単位
     - 意味
   * - ``lag_l0``
     - m
     - 遅れの和 :math:`L_0=(2/S)\int A\,\mathrm{d}x`\ （胴体とフィンの見かけの質量）。見かけの質量の遅れ（:math:`\dot\alpha`）と対流の回転の力 :math:`-\tfrac12\rho SL_0(\cdot)` の大きさ
   * - ``lag_l1``
     - m\ :sup:`2`
     - 遅れの和 :math:`L_1=(2/S)\int A\,x\,\mathrm{d}x`。その力のノーズ先端まわりの能率（中心は :math:`L_1/L_0`）
   * - ``cl4``
     - 無次元
     - 誘導ロール :math:`\Delta C_l=C_{l4}\,\alpha_t^2\sin4(\phi_c-\phi_f)`\ （軸対称表だけ。一般表では :math:`C_l` の列に含める）
   * - ``cnpa``
     - 1/rad
     - Magnus 力 :math:`C_{N,\mathrm{Magnus}}=C_{Np\alpha}^{*}\,\hat p\sin\alpha_t` の傾き。McCoy の :math:`pd/V` で無次元化した :math:`C_{Np\alpha}` とは :math:`C_{Np\alpha}^{*}=2C_{Np\alpha}` の関係にある
   * - ``cmpa``
     - 1/rad
     - 静的な圧力中心まわりの Magnus のモーメントの傾き :math:`C_{mp\alpha}^{*}`\ （:eq:`eq-magnus-moment`）。既定は 0

回転による法線力 :math:`C_{Nq}` は、減衰の和 :math:`S_0,S_1` から作るので、列を持たない（:eq:`eq-cnq-force`）。
線形の交差微係数（:math:`C_{mp},C_{np},C_{mr},C_{nq},C_{lq},C_{lr}`）は、軸対称な胴体と十字形のフィンの機体では対称性から 0 で、どちらの表も列を持たない。
非軸対称な形状の静的な係数は一般表で与えられるが、角速度についての交差微係数は一般表でも表せない（:ref:`sec-lim-motion`）。

各微係数の出どころは次のとおりである。

* **部品積み上げ法**\ ：:math:`S_0,S_1,S_2` を部品の和から（:ref:`sec-damping`）、:math:`C_{l\delta}` と :math:`C_{lp}` を部品積み上げ法のロール項から求める（:ref:`sec-barrowman-roll`）。
* **パネル法**\ ：:math:`S_0,S_1,S_2` を表面の解から求める。ロール項は部品積み上げ法の値である（:doc:`panel`）。
* **CFD**\ ：ピッチとヨーは CFD で直接解かず、他の手法の和を CFD の法線力の傾きに合わせて作る（:math:`C_{mq}` と :math:`C_{nr}` は CFD の値ではない）。
  :math:`C_{lp}` は、設定 ``roll_damping`` が真のとき、回転座標系の CFD（正負のロール角速度の 2 ケースの差）から Mach 数ごとに求める。
  採用するのは負で 0 でない値だけで、失敗したケースや採用できない Mach 数は部品積み上げ法の値を使う。
  表の値は、部品積み上げ法の :math:`C_{lp}` に、CFD と部品積み上げ法の比（Mach 数方向に補間）を掛けたものである。
  ``roll_damping`` が偽のときは部品積み上げ法の値である。:math:`C_{l\delta}` は常に部品積み上げ法の値である（:doc:`cfd`）。
* **外部の表**\ ：表に書かれた値をそのまま使う。ロール項がすべて 0 のときは部品積み上げ法の値を補う（:ref:`sec-aero-external`）。

遅れの和 ``lag_l0``、``lag_l1`` は、部品積み上げ法では形状から求め（:ref:`sec-damping`）、パネル法と CFD の表と、遅れの列を持たない古い形式の外部の表にも同じ形状の値を入れる（:ref:`sec-file-v1`）。
誘導ロール ``cl4`` と Magnus の ``cnpa``、``cmpa`` は、部品積み上げ法、パネル法、CFD のどれも推算せず 0 である。必要なら外部の表で与える（求め方は :ref:`sec-aero-unsteady`）。
実測や強制振動の CFD のピッチ減衰 :math:`C_{mq}+C_{m\dot\alpha}` を使う表では、遅れの和を 0 にして減衰の和に入れる（:ref:`sec-file-v1`）。

確認済み
--------

* 軸対称表から展開した一般表が、軸対称表と同じ空気力とモーメントを与えることを確かめた。
* 6 成分の符号と向き（右手系、ノーズ先端まわりのモーメント）を、CFD の座標変換と併せて確かめた。
* ロール減衰の符号が負であることを CFD で確かめた。

結果は :ref:`sec-vaero-table` にまとめる。

注意
----

* 空力係数は準定常で、角速度と加速度の影響は、減衰の微係数、回転による法線力、見かけの質量の遅れと対流の回転の項、Magnus 力とモーメントで表す。流れの履歴（遅れの状態量）は持たない（:ref:`sec-quasi-steady`）。
* :math:`C_{mq}` と :math:`C_{nr}` は CFD から直接は得られない。
* 一般表の静的な安定は、迎角 0 と横滑り角 0 での傾きから求めたピッチとヨーの余裕の小さいほうで判定し、ピッチとヨーの連成は考えない（:ref:`sec-static-margin-general`）。

詳しくは :ref:`sec-lim-aero` を参照。
