空力解析
========

本章では、抽出した形状から空力係数を計算する方法を説明する。
手法は Barrowman による部品積み上げ法 :cite:`barrowman` を基礎とし、
遷音速・超音速への拡張と抗力の推算には Niskanen :cite:`niskanen,niskanentech` がまとめた方法（OpenRocket の基礎）を用いる。
細長い飛翔体の空力の一般論は Nielsen :cite:`nielsenbook`、圧縮性流れの基礎は Liepmann と Roshko :cite:`liepmann` や Anderson :cite:`anderson` を参照した。

計算結果は Mach 数 :math:`M` と迎角 :math:`\alpha` の格子上の **係数表** として保存し、飛翔計算ではこの表を内挿して使う。

空力係数の定義
--------------

機体座標系で、空気力を機軸方向の **軸力** :math:`A` と機軸に垂直な **法線力** :math:`N` に分ける（:numref:`fig-body-frame`）。
動圧 :math:`q=\tfrac12\rho V^2` と基準面積 :math:`S=\pi d^2/4` を用いて

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

胴体の法線力
------------

細長体理論
~~~~~~~~~~

細長い回転体が小さな迎角 :math:`\alpha` で飛ぶとき、断面積 :math:`A(x)` が変化する部分に、単位長さあたり

.. math::

   \frac{\dd N}{\dd x} = 2\,q\,\alpha\,\frac{\dd A}{\dd x}

の法線力が生じる（細長体理論 :cite:`munk,ashley`）。断面積が一定の円筒部分には法線力が生じない。

IgnisYeet では、抽出した半径分布を断面ごとの円錐台の連なりとみなし、各円錐台 :math:`k`\ （区間 :math:`[x_k,x_{k+1}]`、長さ :math:`\ell_k`、両端の断面積 :math:`A_k, A_{k+1}`）について積分する。

.. math::
   :label: eq-body-cna

   C_{N\alpha,k} = \frac{2}{S}\int_{x_k}^{x_{k+1}}\frac{\dd A}{\dd x}\,\dd x = \frac{2\,(A_{k+1}-A_k)}{S}

区間始端まわりのモーメントは、部分積分により

.. math::

   \int_0^{\ell_k} x'\,\frac{\dd A}{\dd x'}\,\dd x' = \ell_k A_{k+1} - \int_0^{\ell_k}A\,\dd x' = \ell_k A_{k+1} - V_k

となる（:math:`V_k=\tfrac{\pi\ell_k}{3}(r_k^2+r_kr_{k+1}+r_{k+1}^2)` は円錐台の体積）。したがって各区間の圧力中心は

.. math::
   :label: eq-body-cp

   x_{cp,k} = x_k + \frac{\ell_k A_{k+1}-V_k}{A_{k+1}-A_k}

である。ノーズだけでなく、肩（太くなる段）やボートテール（細くなる部分、:math:`C_{N\alpha,k}<0`）も同じ式で自動的に扱える。
全区間で和をとって

.. math::
   :label: eq-body-sums

   S_0^{\mathrm{body}}=\sum_k C_{N\alpha,k},\qquad S_1^{\mathrm{body}}=\sum_k C_{N\alpha,k}\,x_{cp,k},\qquad S_2^{\mathrm{body}}=\sum_k C_{N\alpha,k}\,x_{cp,k}^2

とおく。:math:`S_2` はピッチ減衰（:ref:`sec-damping`）で使う。
ノーズと円筒からなり後端が平らな機体では、式 :eq:`eq-body-cna` の和は :math:`2(A_{\mathrm{base}}-0)/S=2` となり、
ノーズの形によらず :math:`C_{N\alpha}^{\mathrm{body}}=2` という古典的な結果 :cite:`munk,barrowman` が得られる。

有限の迎角では、ポテンシャル流の寄与を

.. math::

   C_N^{\mathrm{pot}} = S_0^{\mathrm{body}}\,\sin\alpha\cos\alpha

とする（:math:`\alpha\to0` で :math:`S_0^{\mathrm{body}}\alpha` に一致する）。

胴体揚力（粘性横流れ）
~~~~~~~~~~~~~~~~~~~~~~

迎角が大きくなると、胴体を横切る流れがはく離して付加的な法線力が生じる。これを

.. math::
   :label: eq-body-lift

   C_N^{\mathrm{lift}} = K\,\frac{A_p}{S}\,\sin^2\alpha,\qquad K=1.1

で表す :cite:`niskanen,allen`\ 。:math:`A_p` は側面投影面積、作用点はその図心 :math:`x_p` とする。
:math:`\sin^2\alpha` に比例するので小迎角では無視できるが、迎角 10° を超えると効き始め、圧力中心を前に移動させる。

フィンの法線力
--------------

フィンの形状は :numref:`fig-fin-planform` の台形で表す。

.. _fig-fin-planform:

.. figure:: _generated/tikz/fin_planform.*
   :width: 80%

   台形フィンの寸法。:math:`a` 根元翼弦、:math:`b` 端翼弦、:math:`s` スパン、:math:`m` 後退長、:math:`\bar c` 平均空力翼弦。

1 枚の面積、アスペクト比（2 枚を合わせた翼として）、中央翼弦線の後退角は

.. math::

   A_f=\frac{(a+b)\,s}{2},\qquad \mathit{AR}=\frac{2s^2}{A_f},\qquad
   \tan\Gamma_c = \frac{m+\tfrac{b}{2}-\tfrac{a}{2}}{s}

である。圧縮性の係数を :math:`\beta=\sqrt{|1-M^2|}` とする。

亜音速（:math:`M\le0.9`）
~~~~~~~~~~~~~~~~~~~~~~~~~

フィン 1 枚の法線力傾斜は、Diederich の平面翼の式 :cite:`diederich` に Prandtl–Glauert の圧縮性補正 :cite:`glauert` を加えた形で

.. math::
   :label: eq-fin-sub

   C_{N\alpha,1} = \frac{2\pi\,s^2/S}{1+\sqrt{1+\left(\dfrac{\beta\,s^2}{A_f\cos\Gamma_c}\right)^2}}

と表す :cite:`niskanen`\ 。:math:`\beta=1` のとき、これは Barrowman の式

.. math::

   C_{N\alpha}^{\mathrm{fins}} = \frac{4N\,(s/d)^2}{1+\sqrt{1+\left(\dfrac{2\ell_f}{a+b}\right)^2}},\qquad
   \ell_f=\frac{s}{\cos\Gamma_c}

（:math:`N` 枚分、干渉なし）と一致する。

超音速（:math:`M\ge1.5`）
~~~~~~~~~~~~~~~~~~~~~~~~~

超音速では、薄い平板の表面圧力係数を、面の傾き :math:`\eta` について Busemann の 3 次展開 :cite:`barrowmanfin,niskanen`


.. math::

   C_p = K_1\eta + K_2\eta^2 + K_3\eta^3

で表す。:math:`\gamma=1.4` として

.. math::

   K_1=\frac{2}{\beta},\qquad
   K_2=\frac{(\gamma+1)M^4-4\beta^2}{4\beta^4},\qquad
   K_3=\frac{(\gamma+1)M^8+(2\gamma^2-7\gamma-5)M^6+10(\gamma+1)M^4+8}{6\beta^7}

である。迎角 :math:`\alpha` の平板では下面が :math:`\eta=+\alpha`、上面が :math:`\eta=-\alpha` なので、両面の差をとると :math:`K_2` の項は打ち消し合い、

.. math::

   \Delta C_p = 2K_1\alpha + 2K_3\alpha^3

となる（:math:`K_1` の項だけなら Ackeret の線形理論 :math:`4\alpha/\beta` に一致する :cite:`ashley,liepmann`\ ）。
これは 2 次元の結果なので、翼端から広がる Mach 円錐の影響 :cite:`ashley,liepmann` を、矩形翼の線形理論による係数

.. math::

   \eta_{\mathrm{tip}} = \min\!\left(1,\ \max\!\left(0.5,\ 1-\frac{1}{2\beta\,\mathit{AR}}\right)\right)

で近似的に差し引く。したがって超音速でのフィン 1 枚の法線力は

.. math::
   :label: eq-fin-sup

   C_{N,1} = \frac{A_f}{S}\left(2K_1\alpha+2K_3\alpha^3\right)\eta_{\mathrm{tip}}

である。

遷音速（:math:`0.9<M<1.5`）
~~~~~~~~~~~~~~~~~~~~~~~~~~~

遷音速では簡単な理論がないので、迎角 :math:`\alpha` を固定し、:math:`M=0.9` の亜音速の値と :math:`M=1.5` の超音速の値を、
両端の値と :math:`M` についての傾きが一致する **3 次 Hermite 補間** でつなぐ。

.. math::
   :label: eq-hermite

   f(M) = h_{00}(\tau)f_0 + h_{10}(\tau)\,\Delta M\,f_0' + h_{01}(\tau)f_1 + h_{11}(\tau)\,\Delta M\,f_1',\qquad \tau=\frac{M-0.9}{\Delta M},\ \Delta M=0.6

.. math::

   h_{00}=2\tau^3-3\tau^2+1,\quad h_{10}=\tau^3-2\tau^2+\tau,\quad h_{01}=-2\tau^3+3\tau^2,\quad h_{11}=\tau^3-\tau^2

端点の傾き :math:`f_0', f_1'` は数値微分で求める。こうすると係数が Mach 数について滑らかにつながり、飛翔計算で不自然な跳びが生じない。

フィン枚数と胴体干渉
~~~~~~~~~~~~~~~~~~~~

:math:`N` 枚のフィン全体の法線力は、1 枚分に枚数係数と胴体干渉係数 :cite:`pnk,barrowman` を掛けて求める。

.. math::
   :label: eq-fin-total

   C_N^{\mathrm{fins}} = K_{fb}\,k_N\,\frac{N}{2}\,C_{N,1},\qquad K_{fb}=1+\frac{r_t}{s+r_t}

:math:`r_t` はフィン位置の胴体半径である。:math:`N/2` は、迎角面に対してフィンがどの角度にあっても、
:math:`N\le4` なら合計の法線力が :math:`N/2` 枚分になることによる。枚数が多いとフィン同士が干渉するので係数 :math:`k_N` で減らす :cite:`niskanen`\ 。

.. list-table:: 枚数係数 :math:`k_N`
   :header-rows: 1

   * - :math:`N`
     - 1〜4
     - 5
     - 6
     - 7
     - 8
     - 9 以上
   * - :math:`k_N`
     - 1.000
     - 0.948
     - 0.913
     - 0.854
     - 0.810
     - 0.750

フィンの圧力中心
~~~~~~~~~~~~~~~~

平均空力翼弦の長さとその前縁の位置（根元前縁から）は

.. math::

   \bar c = \frac23\left(a+b-\frac{ab}{a+b}\right),\qquad
   x_{\bar c} = \frac{m\,(a+2b)}{3\,(a+b)}

である。フィンの圧力中心を :math:`x_f = x_{le}+x_{\bar c}+\kappa(M)\,\bar c` とし、

.. math::
   :label: eq-fin-cp

   \kappa(M) =
   \begin{cases}
   0.25 & (M\le0.5)\\[2pt]
   \dfrac{\mathit{AR}\,\beta-0.67}{2\,\mathit{AR}\,\beta-1} & (M\ge2)
   \end{cases}

とする :cite:`barrowman,niskanen`\ （超音速側は :math:`[0.25,\,0.5]` に制限）。:math:`0.5<M<2` は式 :eq:`eq-hermite` と同じ Hermite 補間でつなぐ。
:math:`\kappa=0.25` のとき、これは Barrowman のフィン圧力中心 :cite:`barrowman`

.. math::

   x_f = x_{le} + \frac{m(a+2b)}{3(a+b)} + \frac16\left(a+b-\frac{ab}{a+b}\right)

と一致する。超音速ではフィンの圧力中心が後退し、静安定余裕が増える。

全機の法線力と圧力中心
----------------------

以上を足し合わせて

.. math::
   :label: eq-cn-total

   C_N(M,\alpha) = S_0^{\mathrm{body}}\sin\alpha\cos\alpha + C_N^{\mathrm{lift}} + C_N^{\mathrm{fins}}

.. math::
   :label: eq-xcp-total

   x_{cp}(M,\alpha) = \frac{S_1^{\mathrm{body}}\sin\alpha\cos\alpha + C_N^{\mathrm{lift}}x_p + C_N^{\mathrm{fins}}x_f}{C_N}

とする。:math:`\alpha=0` では :math:`C_N=0` で式 :eq:`eq-xcp-total` が 0/0 になるので、表には :math:`\alpha\to0` の極限
:math:`(S_1^{\mathrm{body}}+C_{N\alpha}^{\mathrm{fins}}x_f)/(S_0^{\mathrm{body}}+C_{N\alpha}^{\mathrm{fins}})` を書き込む。
法線力傾斜 :math:`C_{N\alpha}` は :math:`\alpha=10^{-4}` rad での差分で求める。

サンプル機体の結果を :numref:`fig-aero-cna-xcp` と :numref:`fig-aero-cn-alpha` に示す。
遷音速で :math:`C_{N\alpha}` がピークを持ち、超音速では :math:`\beta` が大きくなるにつれて減少する。

.. _fig-aero-cna-xcp:

.. figure:: _generated/plots/aero_cna_xcp.*
   :width: 100%

   サンプル機体の法線力傾斜（左）と圧力中心（右）。点線の間が遷音速の補間区間である。

.. _fig-aero-cn-alpha:

.. figure:: _generated/plots/aero_cn_alpha.*
   :width: 80%

   迎角と法線力係数。迎角が大きくなると胴体揚力 :math:`\propto\sin^2\alpha` が加わり、直線からずれる。

.. _sec-damping:

ピッチ減衰
----------

機体が角速度 :math:`\omega` でピッチ回転すると、重心から :math:`\xi_i=x_i-x_{cg}` 離れた部品には
局所的な迎角 :math:`\Delta\alpha_i=\omega\,\xi_i/V` が加わる。そこに生じる法線力
:math:`q S\,C_{N\alpha,i}\,\Delta\alpha_i` が重心まわりに作るモーメントは :cite:`niskanen`、回転を妨げる向きに

.. math::
   :label: eq-damping

   M_{\mathrm{damp}} = -\frac{qS}{V}\,\omega\sum_i C_{N\alpha,i}\,(x_i-x_{cg})^2
   = -\frac{qS}{V}\,\omega\left(S_2-2x_{cg}S_1+x_{cg}^2S_0\right)

となる。ここで :math:`S_n=\sum_i C_{N\alpha,i}\,x_i^n`\ （胴体の和 :eq:`eq-body-sums` にフィンの寄与を加えたもの）である。
重心は燃焼とともに移動するので、表には :math:`x_{cg}` に依存しない :math:`S_0,S_1,S_2` を Mach 数ごとに保存し、
飛翔計算のたびに式 :eq:`eq-damping` を組み立てる。括弧内が負になる場合（ボートテールの負の寄与が勝つ場合）は 0 とする。

抗力（軸力）
------------

迎角 0 での抗力係数 :math:`C_{D0}` を、次の成分の和として求める :cite:`niskanen,hoerner`\ 。

.. math::
   :label: eq-cd0

   C_{D0} = C_{D,f} + C_{D,\mathrm{nose}} + C_{D,\mathrm{base}} + C_{D,\mathrm{fin}} + C_{D,\mathrm{bt}} + C_{D,\mathrm{extra}}

表面摩擦
~~~~~~~~

Reynolds 数を全長 :math:`L` 基準で :math:`Re=VL/\nu` とする。
:math:`V` と動粘性係数 :math:`\nu` は射点高度の標準大気での値とし、:math:`V=\max(M,0.05)\,a`\ （極低速で発散しないように下限を設ける）とする。
摩擦係数は、乱流の式と粗さで頭打ちになる式の大きい方をとる :cite:`niskanen,schlichting,hoerner`。

.. math::
   :label: eq-cf

   C_f^{\mathrm{turb}}=\frac{1}{(1.50\ln Re-5.6)^2},\qquad
   C_f^{\mathrm{rough}}=0.032\left(\frac{R_s}{L}\right)^{0.2},\qquad
   C_f=\max\!\left(C_f^{\mathrm{turb}},\,C_f^{\mathrm{rough}}\right)

ただし :math:`Re<10^4` では :math:`C_f=1.48\times10^{-2}` とする。:math:`R_s` は等価砂粒粗さ（設定 ``roughness``）である。
圧縮性の補正 :cite:`niskanen` として

.. math::

   C_f^{\mathrm{turb}} \leftarrow
   \begin{cases} C_f^{\mathrm{turb}}(1-0.1M^2) & (M<1)\\ C_f^{\mathrm{turb}}/(1+0.15M^2)^{0.58} & (M\ge1)\end{cases},\qquad
   C_f^{\mathrm{rough}} \leftarrow
   \begin{cases} C_f^{\mathrm{rough}}(1-0.1M^2) & (M<1)\\ C_f^{\mathrm{rough}}/(1+0.18M^2) & (M\ge1)\end{cases}

を掛ける。:numref:`fig-skin-friction` に :math:`Re` と粗さによる変化を示す。サンプル機体（:math:`L=1.5` m、60 µm）では :math:`Re\sim10^7` で粗さの式が支配的になる。

.. _fig-skin-friction:

.. figure:: _generated/plots/skin_friction.*
   :width: 80%

   表面摩擦係数。Reynolds 数が大きくなると、粗さで決まる一定値で頭打ちになる。

摩擦抗力は、胴体の細長比 :math:`f_B=L/d` とフィンの厚み比 :math:`t/\bar c` による形状補正 :cite:`hoerner,niskanen` を加えて

.. math::
   :label: eq-cdf

   C_{D,f} = \frac{C_f}{S}\left[\left(1+\frac{1}{2f_B}\right)S_{\mathrm{wet}} + \left(1+\frac{2t}{\bar c}\right)2N A_f\right]

とする（:math:`2NA_f` はフィン両面の濡れ面積）。

.. _sec-nose-drag:

ノーズの圧力抗力・造波抗力
~~~~~~~~~~~~~~~~~~~~~~~~~~

亜音速（:math:`M\le0.8`）では、ノーズと胴体の接合部の半頂角 :math:`\phi_j` を用いて :math:`C_D=0.8\sin^2\phi_j` とする :cite:`hoerner,niskanen`。
接線オジブのように滑らかにつながるノーズでは、ほぼ 0 になる。

超音速（:math:`M\ge1.3`）では、等価な円錐の半頂角 :math:`\phi_c=\arctan(R_n/L_n)` について円錐の造波抗力の近似式 :cite:`niskanen` を用い、
ノーズの形の違いを体積比 :math:`v_r` による係数 :math:`k_s` で表す。

.. math::
   :label: eq-nose-sup

   C_D = k_s\left(2.1\sin^2\phi_c + \frac{0.5\sin\phi_c}{\sqrt{M^2-1}}\right),\qquad
   k_s = \begin{cases} 1 & (v_r\le1/3)\\ \max\!\left(0.6,\ 1-0.4\,\dfrac{v_r-1/3}{0.55-1/3}\right) & (v_r>1/3)\end{cases}

円錐（:math:`v_r=1/3`）では :math:`k_s=1`、オジブのようにふくらんだノーズほど造波抗力が小さくなる。
:math:`0.8<M<1.3` は Hermite 補間でつなぐ（:math:`M=0.8` での傾きは 0）。いずれもノーズ底面積 :math:`\pi R_n^2/S` を掛ける。
:math:`k_s` は形状の違いを粗く表すための近似であり、ノーズ抗力は本モデルで最も不確かさが大きい部分の一つである。

底面抗力
~~~~~~~~

機体後端の平らな面には負圧がかかる :cite:`hoerner,niskanen`。底面積あたりの係数を

.. math::
   :label: eq-base

   C_{D,b}(M) = \begin{cases} 0.12+0.13M^2 & (M<1)\\ 0.25/M & (M\ge1)\end{cases}

とし、底面積 :math:`\pi r_{\mathrm{base}}^2` を掛ける。燃焼中は噴流が底面を埋めるので、ノズル出口面積 :math:`A_e` を差し引いた
:math:`\pi r_{\mathrm{base}}^2-A_e` を使う。このため係数表には燃焼中（power-on）と燃焼後（power-off）の 2 種類の :math:`C_A` がある。

フィンの前縁・後縁
~~~~~~~~~~~~~~~~~~

丸めた前縁の抗力 :cite:`hoerner,niskanen`\ （前縁に垂直な成分、前縁の正面面積 :math:`Nst` あたり）は

.. math::

   C_{D,\mathrm{LE}\perp} =
   \begin{cases}
   (1-M^2)^{-0.417}-1 & (M<0.9)\\
   1-1.785\,(M-0.9) & (0.9\le M<1)\\
   1.214-\dfrac{0.502}{M^2}+\dfrac{0.1095}{M^4} & (M\ge1)
   \end{cases}

で、前縁の後退角 :math:`\Gamma_{LE}=\arctan(m/s)` について :math:`\cos^2\Gamma_{LE}` を掛ける。
3 つの式は :math:`M=0.9` と :math:`M=1` で連続につながる。後縁を切り落とした（square）フィンには、後縁の面積 :math:`Nst` に式 :eq:`eq-base` の底面抗力を加える。

ボートテール
~~~~~~~~~~~~

ボートテールの長さ :math:`L_{bt}` と前後の直径差の比 :math:`\gamma=L_{bt}/(d_f-d_a)` によって

.. math::

   C_{D,\mathrm{bt}} = k_{bt}\,C_{D,b}(M)\,\frac{\pi(r_f^2-r_a^2)}{S},\qquad
   k_{bt} = \begin{cases} 1 & (\gamma<1)\\ (3-\gamma)/2 & (1\le\gamma\le3)\\ 0 & (\gamma>3)\end{cases}

とする。緩やかに絞ったボートテールほど抗力が小さい。

抗力の内訳
~~~~~~~~~~

サンプル機体の抗力の内訳を :numref:`fig-aero-drag` に示す。
亜音速では表面摩擦が半分以上を占める。遷音速では底面抗力・フィン前縁の抗力・ノーズの造波抗力が増え、抗力係数は最大になる。

.. _fig-aero-drag:

.. figure:: _generated/plots/aero_drag.*
   :width: 100%

   サンプル機体の零揚力抗力係数の内訳（燃焼後）。破線は燃焼中の合計。

迎角による軸力の変化
~~~~~~~~~~~~~~~~~~~~

迎角があると軸力は変化する。OpenRocket と同じく :cite:`niskanen`、:math:`\alpha=17^\circ` で 1.3 倍の最大値をとり、:math:`90^\circ` で 0 になる
区分的な 3 次多項式（両端の傾き 0 の Hermite 補間）を掛ける（:numref:`fig-axial-factor`）。

.. math::
   :label: eq-ca-alpha

   C_A(M,\alpha) = C_{D0}(M)\,f(\alpha)

.. _fig-axial-factor:

.. figure:: _generated/plots/axial_factor.*
   :width: 80%

   迎角による軸力係数の倍率 :math:`f(\alpha)`。

係数表と内挿・外挿
------------------

格子と保存形式
~~~~~~~~~~~~~~

既定では :math:`M=0,0.02,\dots,3.0`\ （151 点）、:math:`\alpha=0^\circ,1^\circ,\dots,30^\circ`\ （31 点）の格子で、
各点の :math:`C_N`, :math:`C_A^{\mathrm{on}}`, :math:`C_A^{\mathrm{off}}`, :math:`x_{cp}`, :math:`C_{N\alpha}`, :math:`S_0`, :math:`S_1`, :math:`S_2`
を ``aero_table.csv`` に保存する。基準量・格子・入力のハッシュ値は ``aero_table.json`` に保存する。
CSV なので、風洞試験や CFD で得た係数に差し替えて使うこともできる。

双線形内挿
~~~~~~~~~~

飛翔中の :math:`(M,\alpha)` が格子の区間 :math:`[M_i,M_{i+1}]\times[\alpha_j,\alpha_{j+1}]` にあるとき（:numref:`fig-table-interp`）、

.. math::

   t_M = \frac{M-M_i}{M_{i+1}-M_i},\qquad t_\alpha=\frac{\alpha-\alpha_j}{\alpha_{j+1}-\alpha_j}

として、四隅の値 :math:`c_{00},c_{10},c_{01},c_{11}` から

.. math::
   :label: eq-bilinear

   c(M,\alpha) = (1-t_M)(1-t_\alpha)\,c_{00} + t_M(1-t_\alpha)\,c_{10} + (1-t_M)\,t_\alpha\,c_{01} + t_M\,t_\alpha\,c_{11}

を求める。

.. _fig-table-interp:

.. figure:: _generated/tikz/table_interp.*
   :width: 70%

   係数表の双線形内挿と、範囲外での線形外挿。

外挿
~~~~

:math:`(M,\alpha)` が表の範囲外にあるときは、最も近い端の区間を使い、:math:`t_M` や :math:`t_\alpha` が :math:`[0,1]` を外れたまま
式 :eq:`eq-bilinear` を評価する。これは端の区間の傾きをそのまま延長する **線形外挿** である（設定 ``extrapolation = "linear"``）。
``"clamp"`` を選ぶと :math:`t` を :math:`[0,1]` に制限し、端の値で一定とする。
外挿で軸力係数が負にならないよう、:math:`C_A` は 0 以上に制限する。

迎角は :math:`0\le\alpha\le\pi` の全迎角で与えられる。機体が後ろ向きに飛ぶ（:math:`\alpha>90^\circ`）場合は、
前向きの係数を :math:`\alpha'=\pi-\alpha` で引き、軸力の向きを反転させる（:ref:`sec-aero-forces`）。
