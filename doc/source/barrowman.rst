.. _page-theory-barrowman:

部品積み上げ法
================

部品積み上げ法は、``aero.method = "barrowman"``\ （既定）で選ぶ空力解析である。
機体をノーズ・胴体・フィン・ボートテールといった部品に分け、部品ごとに法線力・作用点・抗力を求めて足し合わせる。
計算は表全体で 1 秒未満、外部のプログラムも要らないので、形状を変えながら設計を進める段階で最初に使う手法になる。
一方、使う入力は抽出した半径分布とフィンの平面形、``rocket.fin_le``、``rocket.fin_te``、``rocket.fin_cant_deg``、``rocket.roughness``、``rocket.extra_cd``\ （:ref:`page-theory-geometry`）だけで、表面の形そのものは使わない。
その結果、実測に対して法線力と軸力が小さく出る向きに偏る。

手法は Barrowman による部品積み上げ法 :cite:`barrowman` を基礎とし、遷音速・超音速への拡張と抗力の推算には Niskanen :cite:`niskanen,niskanentech` がまとめた方法（OpenRocket の基礎）を用いる。
細長い飛翔体の空力の一般論は Nielsen :cite:`nielsenbook`、圧縮性流れの基礎は Liepmann と Roshko :cite:`liepmann` や Anderson :cite:`anderson` を参照した。
この章は、胴体、フィン、全機の法線力と圧力中心、減衰とロール、抗力の順に式を示し、適用範囲と実測との差を述べたあと、入力の感度から選び方をまとめる。
手法どうしの使い分けは :ref:`page-theory-aero-choose` に、係数の定義は :ref:`page-theory-aero-coefficients` にある。

モデル
------

部品積み上げ法とは
~~~~~~~~~~~~~~~~~~

**何を求めるか**

この手法が求めるのは、迎角のついた機体に働く法線力、その作用点である圧力中心、軸力（迎角 0 では抗力）、
角速度に比例する減衰、およびフィンのカントによるロールの量である。
これらを Mach 数と迎角の関数として計算し、係数表に書き込む（:ref:`sec-aero-table`）。
法線力 :math:`C_N` は機軸に垂直な力、軸力 :math:`C_A` は機軸方向の力の係数、:math:`x_{cp}` はノーズ先端から測った圧力中心である。
:math:`\alpha` は機軸と対気速度のなす角（全迎角）で、定義は :ref:`page-theory-aero-coefficients` にある。

**物理的な直感**

細長い機体では、空気が側面を斜めに流れるとき、法線力が生じる場所は限られる。
胴体は、断面積が変化する場所（ノーズ、肩、ボートテール）で法線力を生み、断面が一定の円筒部ではほとんど生まない。
太くなる部分は正、細くなる部分は負の寄与である。
フィンは小さな翼として働き、迎角に比例した揚力を機体の後方で生む。この揚力が、圧力中心を後ろへ引き寄せる。
各部品の法線力がそれぞれの作用点で機体を押すと考えると、全体の圧力中心は、法線力を重みにした作用点の平均になる（:numref:`fig-barrowman-contrib`）。

.. _fig-barrowman-contrib:

.. figure:: /_generated/tikz/barrowman_contrib.*
   :width: 100%

   部品ごとの法線力の寄与（模式図）。断面積 :math:`A` が増える部分は正、一定の部分はほぼ 0、減る部分は負の寄与をする。
   フィンは翼として揚力を生む。:math:`x_i` はノーズ先端から測った各部品の作用点で、:math:`x_{cp}` は法線力を重みにした平均である。

**モデルの構成**

胴体は、抽出した半径分布を円錐台の連なりとみなして扱う。ノーズ・肩・ボートテールは、断面積の変化として同じ式で足し合わされる。
フィンは、根元翼弦・端翼弦・スパン・後退長で決まる台形で表し、Mach 数に応じて亜音速・超音速・遷音速の式を使い分ける。
全機の圧力中心の和は :eq:`eq-cp-sum`\ （:ref:`法線力・軸力・圧力中心 <sec-aero-xcp>`）で定義しており、本章はその各項の求め方を述べる。
抗力は表面摩擦・ノーズ・底面・フィンの縁・ボートテールの成分の和で、迎角による変化を掛けて軸力にする。
ピッチ減衰は、部品ごとの法線力の傾きの和から作る。

胴体の法線力
~~~~~~~~~~~~

ノーズが太くなる部分と肩は正、ボートテールは負の寄与になる。
半径分布を円錐台に区切り、区間ごとの断面積の変化から法線力の傾きと作用点を求めて、全区間で足し合わせる。
有限の迎角では、ポテンシャル流の寄与に、はく離した横流れによる胴体揚力を加える。

**細長体理論**

細長い回転体が小さな迎角 :math:`\alpha` で飛ぶとき、断面積 :math:`A(x)` が変化する部分に、単位長さあたり

.. math::

   \frac{\dd N}{\dd x} = 2\,q\,\alpha\,\frac{\dd A}{\dd x}

の法線力が生じる（細長体理論 :cite:`munk,ashley`）。断面積が一定の円筒部分には法線力が生じない。

抽出した半径分布を断面ごとの円錐台の連なりとみなし、各円錐台 :math:`k`\ （区間 :math:`[x_k,x_{k+1}]`、長さ :math:`\ell_k`、両端の断面積 :math:`A_k, A_{k+1}`）について積分する。

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
ノーズと円筒からなり後端が平らな機体では、\ :eq:`eq-body-cna` の和は :math:`2(A_{\mathrm{base}}-0)/S=2` となり、
ノーズの形によらず :math:`C_{N\alpha}^{\mathrm{body}}=2` という古典的な結果 :cite:`munk,barrowman` が得られる。

有限の迎角では、ポテンシャル流の寄与を

.. math::

   C_N^{\mathrm{pot}} = S_0^{\mathrm{body}}\,\sin\alpha\cos\alpha

とする（:math:`\alpha\to0` で :math:`S_0^{\mathrm{body}}\alpha` に一致する）。

**胴体揚力（粘性横流れ）**

迎角が大きくなると、胴体を横切る流れがはく離して付加的な法線力が生じる。これを

.. math::
   :label: eq-body-lift

   C_N^{\mathrm{lift}} = K\,\frac{A_p}{S}\,\sin^2\alpha,\qquad K=1.1

で表す :cite:`niskanen,allen`\ 。:math:`A_p` は側面投影面積、作用点はその図心 :math:`x_p` とする。
:math:`\sin^2\alpha` に比例するので小迎角では無視できるが、迎角 10° を超えると効き始め、圧力中心を前に移動させる。

フィンの法線力
~~~~~~~~~~~~~~

フィンは、機体に付いた小さな翼として働く。迎角がつくと揚力が生じる。
翼の揚力は Mach 数によって性質が変わるので、亜音速・超音速・遷音速で別の式を使う。
亜音速では平面翼の式に圧縮性の補正を加え、超音速では平板の表面圧力から求め、その間の遷音速は両側の値を滑らかにつなぐ。
こうして求めた 1 枚分の法線力に、枚数と胴体との干渉の係数を掛けて、フィン全体の法線力にする。
フィンの圧力中心も Mach 数で変わり、超音速では後退する。

フィンの形状は :numref:`fig-fin-planform` の台形で表す。

.. _fig-fin-planform:

.. figure:: /_generated/tikz/fin_planform.*
   :width: 80%

   台形フィンの寸法。:math:`a` 根元翼弦、:math:`b` 端翼弦、:math:`s` スパン、:math:`m` 後退長、:math:`\bar c` 平均空力翼弦。見る点は、``fin_override`` の ``root_chord``、``tip_chord``、``span``、``sweep`` がこの寸法に対応することである。

1 枚の面積、アスペクト比（2 枚を合わせた翼として）、中央翼弦線の後退角は

.. math::

   A_f=\frac{(a+b)\,s}{2},\qquad \mathit{AR}=\frac{2s^2}{A_f},\qquad
   \tan\Gamma_c = \frac{m+\tfrac{b}{2}-\tfrac{a}{2}}{s}

である。圧縮性の係数を :math:`\beta=\sqrt{|1-M^2|}` とする。
この :math:`\beta` は、:ref:`page-theory-aero-coefficients` の横滑り角 :math:`\beta` とは別の量である。
同じ記号を使うが、本章（とパネル法）では圧縮性の係数だけを指す。

**亜音速（:math:`M\le0.9`）**

フィン 1 枚の法線力傾斜は、Diederich の平面翼の式 :cite:`diederich` に Prandtl–Glauert の圧縮性補正 :cite:`glauert` を加えた形で

.. math::
   :label: eq-fin-sub

   C_{N\alpha,1} = \frac{2\pi\,s^2/S}{1+\sqrt{1+\left(\dfrac{\beta\,s^2}{A_f\cos\Gamma_c}\right)^2}}

と表す :cite:`niskanen`\ 。:math:`\beta=1` のとき、これは Barrowman の式

.. math::

   C_{N\alpha}^{\mathrm{fins}} = \frac{4N\,(s/d)^2}{1+\sqrt{1+\left(\dfrac{2\ell_f}{a+b}\right)^2}},\qquad
   \ell_f=\frac{s}{\cos\Gamma_c}

（:math:`N` 枚分、干渉なし）と一致する。

**超音速（:math:`M\ge1.5`）**

超音速では、薄い平板の表面圧力係数を、面の傾き :math:`\eta` について Busemann の 3 次展開 :cite:`barrowmanfin,niskanen`


.. math::

   C_p = K_1\eta + K_2\eta^2 + K_3\eta^3

で表す。:math:`\gamma=1.4` として

.. math::

   K_1&=\frac{2}{\beta},\qquad
   K_2=\frac{(\gamma+1)M^4-4\beta^2}{4\beta^4},\\
   K_3&=\frac{(\gamma+1)M^8+(2\gamma^2-7\gamma-5)M^6+10(\gamma+1)M^4+8}{6\beta^7}

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

**遷音速（:math:`0.9<M<1.5`）**

遷音速では簡単な理論がないので、迎角 :math:`\alpha` を固定し、:math:`M=0.9` の亜音速の値と :math:`M=1.5` の超音速の値を、
両端の値と :math:`M` についての傾きが一致する **3 次 Hermite 補間** でつなぐ。

.. math::
   :label: eq-hermite

   f(M) = h_{00}(\tau)f_0 + h_{10}(\tau)\,\Delta M\,f_0' + h_{01}(\tau)f_1 + h_{11}(\tau)\,\Delta M\,f_1',\qquad \tau=\frac{M-0.9}{\Delta M},\ \Delta M=0.6

.. math::

   h_{00}=2\tau^3-3\tau^2+1,\quad h_{10}=\tau^3-2\tau^2+\tau,\quad h_{01}=-2\tau^3+3\tau^2,\quad h_{11}=\tau^3-\tau^2

端点の傾き :math:`f_0', f_1'` は両端の式の :math:`M` についての傾きである。こうすると係数が Mach 数について滑らかにつながり、飛翔計算で不自然な跳びが生じない。

**フィン枚数と胴体干渉**

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

:math:`K_{fb}` は、胴体がフィンの影響で受ける揚力の増加（フィンの揚力を胴体が受ける分、:math:`K_{B(W)}`）を含まない。
サンプル機体（:math:`r_t/(s+r_t)=1/3`）では、細長体理論の :math:`K_{W(B)}+K_{B(W)}` が約 1.78、この式の :math:`K_{fb}` が 1.33 で、
フィン部分の :math:`C_{N\alpha}` は約 25 % 小さい（パネル法と CFD との差は :ref:`sec-vaero-compare`、影響は :ref:`sec-lim-aero`）。
静安定余裕は圧力中心が前に出るので安全側になるが、風見効果と落下点は変わる。落下分散では、``aero.method = "panel"`` との比較と、``cn_scale`` の標準偏差で差を含めるとよい。

.. _sec-fin-large-alpha:

大迎角のフィンの法線力
~~~~~~~~~~~~~~~~~~~~~~

ここまでのフィンの式は、流れがフィンに付着している小さな迎角の理論である。
超音速の Busemann の展開は衝撃波が前縁に付いている間だけ成り立ち、衝撃波は迎角が最大の転向角（:math:`M=1.5` で約 12°、:math:`M=2` で約 23°）を超えると離れる。
OpenRocket のフィンのモデルも、フィンの迎角を失速角 20° で打ち切る :cite:`niskanen`。
そこで、フィン全体の法線力 :math:`C_N^{\mathrm{fins}}`\ （:eq:`eq-fin-total`）を、:math:`\alpha_1=15^\circ` まではここまでの式のまま使い、
それより大きな迎角では、フィンを流れに垂直な平板とみなした横流れの値へなめらかに近づける。
:math:`\alpha_1` までの式は変えないので、:math:`C_{N\alpha}` と小さい迎角の係数は変わらない。:math:`\alpha_1=15^\circ` では、Busemann の 3 次の項は 1 次の項の約 2 割までにとどまる（遷音速の補間を通して :math:`M\approx1.2` で最大）。

:math:`90^\circ` での値は、平板の垂直抗力から

.. math::
   :label: eq-fin-cn90

   C_{N,90}^{\mathrm{fins}} = k_N\,\frac{N}{2}\,C_{D,\mathrm{plate}}(M)\,\frac{A_f}{S},\qquad
   C_{D,\mathrm{plate}} = \begin{cases}1.2 & (M\le0.8)\\ 1.7 & (M\ge1.2)\end{cases}

とする（:math:`A_f` はフィン 1 枚の面積）。:math:`0.8<M<1.2` は、両端の傾きを 0 とする 3 次 Hermite 補間でつなぐ。
亜音速の 1.2 は、縦横比 1〜5 の平板の垂直抗力係数 1.17〜1.2、超音速の 1.7 は、流れに垂直な平板と円板の 1.6〜1.8 の範囲の値である :cite:`hoerner`。
胴体干渉係数 :math:`K_{fb}` は小さい迎角のポテンシャル流の吹き上げを表すので、この横流れの値には掛けない。

:math:`\alpha_1<\alpha\le90^\circ` では、:math:`t=\alpha-\alpha_1`、:math:`T=90^\circ-\alpha_1`、:math:`s=t/T`、:math:`\Delta=C_{N,90}^{\mathrm{fins}}-C_{N,1}` として

.. math::
   :label: eq-fin-crossflow

   C_N^{\mathrm{fins}} = C_{N,1} + \Delta\,\frac{1-e^{-u\tau/T}}{1-e^{-u}} + (d_0-d_e)\,b(t),\qquad
   \tau = T\sin\frac{\pi t}{2T},\qquad b(t)=t\,(1-s)^2

とする。:math:`C_{N,1}` と :math:`d_0` は :math:`\alpha_1` での付着流の値と傾きである。
:math:`b` は両端で 0 になるこぶ（:math:`b'(0)=1`、:math:`b'(T)=0`）、:math:`d_e` は指数関数の項の :math:`t=0` での傾きで、

.. math::
   :label: eq-fin-crossflow-slope

   d_e=\frac{\pi}{2}\,\frac{\Delta}{T}\,g(u),\qquad g(u)=\frac{u}{1-e^{-u}}

である。:math:`\Delta>0` では、:math:`u` を :math:`g(u)=d_0T/(\Delta\pi/2)` の解として数値的に決める。このとき :math:`d_e=d_0` で、こぶの項は消え、
曲線は :math:`\alpha_1` で値と傾きが連続で、:math:`C_{N,90}^{\mathrm{fins}}` まで単調に増え、それを超えない（:math:`u` の正負によらない）。指数関数で担えない傾きの残りは、こぶの項が受け持つ。
:math:`\Delta\le0`\ （付着流の値がすでに横流れの値以上。縦横比の大きなフィン）では、指数関数の項を :math:`\Delta\,(3s^2-2s^3)` に置き換える。
式全体は、両端の傾きを :math:`d_0` と 0 にした 3 次 Hermite 補間になり、有界な行き過ぎのあと :math:`C_{N,90}^{\mathrm{fins}}` に戻る。
どちらの場合も、:math:`t=0` で値と傾きが付着流に一致し、:math:`t=T` で値が :math:`C_{N,90}^{\mathrm{fins}}`、傾きが 0 になる（:math:`\tau` によって :math:`90^\circ` での傾きは 0 になる。平板は :math:`90^\circ` について対称）。
:math:`\Delta\to0` では、:math:`u` が有界なので指数関数の項と :math:`d_e` は :math:`\Delta` の有界な倍数で、どちらの式も :math:`C_{N,1}+d_0\,b(t)` に近づく。
そのため、:math:`C_N^{\mathrm{fins}}` は :math:`C_{N,1}`、:math:`C_{N,90}^{\mathrm{fins}}`、:math:`d_0` について連続で、付着流の値が横流れの値を横切る Mach 数や形状でも跳ばない。
この形（:math:`\alpha_1` の上で折れ、横流れの値で頭打ちになる）は、縦横比の小さい平板の法線力の測定値（面積あたり 30° で約 1.0、45〜90° で 1.1〜1.2）の傾向に沿う :cite:`hoerner`。
:math:`90^\circ` を超える迎角（後ろ向きの流れ）では :math:`180^\circ-\alpha` の値を使い、:math:`C_N^{\mathrm{fins}}` は :math:`\alpha` について奇関数である。

フィンの圧力中心も、:math:`\alpha_1` までは :eq:`eq-fin-cp` のままで、それより大きい迎角では、平均空力翼弦に対する位置 :math:`\kappa` を、
両端の傾きを 0 とする 3 次 Hermite 補間で :math:`90^\circ` の 0.5（台形の平面形の図心。流れに垂直な平板の圧力はこの点について対称）へ動かす。
係数表の :math:`90^\circ` を超える行（``aero.alpha_max_deg`` が 90 を超える表だけにある）には :math:`1-\kappa(180^\circ-\alpha)` を入れ、亜音速の :math:`180^\circ` では翼弦の 3/4 になる。ただし、飛翔計算はこの行を読まない。軸対称表の飛翔モデル、6 成分の引き出し、一般表への展開は、全迎角を :math:`\min(\alpha,\,180^\circ-\alpha)` に折り返して表を引くので、尾部を先にした飛翔は、前向きの係数（圧力中心を含む）を :math:`90^\circ` について折り返して使う（:ref:`sec-aero-table`）。

サンプル機体の全体の :math:`C_N`\ （胴体を含む）は、:math:`\alpha=90^\circ` で :math:`M=0.3`、1.2、2.0 のとき 24.2、26.1、26.1 である。
大迎角のフィンの法線力は経験的な近似で、平板の値の幅（亜音速で 1.17〜2.0、超音速で 1.6〜1.8）と遷音速のつなぎ方の分だけ不確かである（:ref:`sec-lim-aero`）。
パネル法の亜音速の列も、:math:`\alpha_1` から :math:`2\alpha_1=30^\circ` の間でこのモデルの全体の法線力と圧力中心へ移り、:math:`30^\circ` から先はこのモデルの値を使う（:eq:`eq-pn-large-alpha`）。CFD の表も、解いた迎角より大きい範囲でこのモデルへつなぐ（:ref:`page-theory-cfd-coefficients`）。

**フィンの圧力中心**

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
   \kappa_s(\mathit{AR}\,\beta) & (M\ge2)
   \end{cases}

とし、:math:`0.5<M<2` は\ :eq:`eq-hermite` と同じ形の Hermite 補間でつなぐ（:math:`M=0.5` での傾きは 0、:math:`M=2` での傾きは式の傾き）\ :cite:`barrowman,niskanen`。
:math:`\mathit{AR}=2s^2/A_f` は向かい合う 1 組のフィンの縦横比である。超音速の位置 :math:`\kappa_s` は :math:`x=\mathit{AR}\,\beta` の関数として

.. math::
   :label: eq-fin-cp-supersonic

   \kappa_s(x) =
   \begin{cases}
   0.25 & (x\le0.84)\\[2pt]
   \text{3 次 Hermite 補間} & (0.84<x<1.2)\\[2pt]
   \min\!\left(\dfrac{x-0.67}{2x-1},\ 0.5\right) & (x\ge1.2)
   \end{cases}

とする。:math:`x\ge1.2` の式は Fleeman :cite:`fleeman` の式で、Niskanen :cite:`niskanen` が :math:`M\ge2` に使うものである。:math:`x=1.2` の約 0.38 から、:math:`x\to\infty` で 0.5 に近づく。
この式は :math:`x=0.5` に極を持ち、:math:`x=0.84` で翼弦の 1/4 まで下がるので、小さい :math:`x` には使わない。
縦横比の小さい翼の荷重分布は Mach 数によらない（細長翼の極限 :cite:`jones835`）ので、:math:`x\le0.84` では圧力中心を亜音速の位置（翼弦の 1/4）に保つ。
:math:`0.84<x<1.2` は、:math:`x=0.84` で傾き 0、:math:`x=1.2` で式の傾き :math:`0.34/(2x-1)^2` をとる単調な 3 次 Hermite 補間でつなぐ。
:math:`\kappa_s` は :math:`[0.25,\,0.5]` の中にあり、Mach 数とフィンの形状について連続である。

:math:`\kappa=0.25` のとき、これは Barrowman のフィン圧力中心 :cite:`barrowman`

.. math::

   x_f = x_{le} + \frac{m(a+2b)}{3(a+b)} + \frac16\left(a+b-\frac{ab}{a+b}\right)

と一致する。超音速ではフィンの圧力中心が後退し、静安定余裕が増える。

全機の法線力と圧力中心
~~~~~~~~~~~~~~~~~~~~~~

胴体の法線力と、胴体揚力、フィンの法線力を足したものが機体全体の法線力になる。
圧力中心は、各部品の作用点を法線力で重みづけした平均として求める。
迎角が 0 のときは法線力が 0 で圧力中心が決まらないので、迎角を 0 に近づけた極限の値を表に書く。
サンプル機体では、遷音速で法線力の傾きにピークが現れ、超音速では減少する。
迎角が大きくなると、胴体揚力が加わって法線力が直線からずれ、圧力中心が前へ動く。

以上を足し合わせて

.. math::
   :label: eq-cn-total

   C_N(M,\alpha) = S_0^{\mathrm{body}}\sin\alpha\cos\alpha + C_N^{\mathrm{lift}} + C_N^{\mathrm{fins}}

.. math::
   :label: eq-xcp-total

   x_{cp}(M,\alpha) = \frac{S_1^{\mathrm{body}}\sin\alpha\cos\alpha + C_N^{\mathrm{lift}}x_p + C_N^{\mathrm{fins}}x_f}{C_N}

とする。これは :eq:`eq-cp-sum` を、本章の部品の式で書いたものである。
:math:`\alpha=0` では :math:`C_N=0` で\ :eq:`eq-xcp-total` が 0/0 になるので、表には :math:`\alpha\to0` の極限
:math:`(S_1^{\mathrm{body}}+C_{N\alpha}^{\mathrm{fins}}x_f)/(S_0^{\mathrm{body}}+C_{N\alpha}^{\mathrm{fins}})` を書き込む。
フィンがなく、尾部が点に閉じた胴体では、胴体の法線力傾斜の和 :math:`S_0^{\mathrm{body}}` が 0 になり、この極限も 0/0 になる。
このときは、迎角 0 の圧力中心を胴体だけの圧力中心と同じくノーズ先端（:math:`x=0`）に置き、表のすべての値を有限に保つ。

サンプル機体の結果を :numref:`fig-aero-cna-xcp` と :numref:`fig-aero-cn-alpha` に示す。
遷音速で :math:`C_{N\alpha}` がピークを持ち、超音速では :math:`\beta` が大きくなるにつれて減少する。

.. _fig-aero-cna-xcp:

.. figure:: /_generated/plots/aero_cna_xcp.*
   :width: 100%

   サンプル機体の法線力傾斜（左）と圧力中心（右）。点線の間が遷音速の補間区間である。見る点は、:math:`C_{N\alpha}` と圧力中心がともに Mach 1.4 付近でピークを持ち、超音速では Mach 数とともに小さくなる（圧力中心は前へ寄る）ことである。

.. _fig-aero-cn-alpha:

.. figure:: /_generated/plots/aero_cn_alpha.*
   :width: 80%

   迎角と法線力係数。迎角が大きくなると胴体揚力 :math:`\propto\sin^2\alpha` が加わり、直線からずれる。
   15° を超えると、フィンの法線力は流れに垂直な平板の横流れの値へ頭打ちになる（:ref:`sec-fin-large-alpha`）。

.. _sec-damping:

ピッチ減衰
~~~~~~~~~~

機体がピッチ回転すると、重心から離れた部品の位置では、回転によって気流の向きが変わって見える。
重心より前の部品には上向きの、後ろの部品には下向きの迎角が加わり、そこに生じる法線力が、回転を妨げる向きのモーメントを作る。
重心から遠い部品ほど、迎角の変化も腕の長さも大きいので、減衰への寄与が大きい（和に距離の 2 乗が入る）。
部品ごとの法線力の傾きと位置の和を Mach 数ごとに表へ保存し、飛翔計算のときに、そのときの重心位置から減衰のモーメントを組み立てる。

機体が角速度 :math:`\omega` でピッチ回転すると、重心から :math:`\xi_i=x_i-x_{cg}` 離れた部品には
局所的な迎角 :math:`\Delta\alpha_i=\omega\,\xi_i/V` が加わる。そこに生じる法線力
:math:`q S\,C_{N\alpha,i}\,\Delta\alpha_i` が重心まわりに作るモーメントは :cite:`niskanen`、回転を妨げる向きに

.. math::
   :label: eq-damping

   M_{\mathrm{damp}} = -\frac{qS}{V}\,\omega\sum_i C_{N\alpha,i}\,(x_i-x_{cg})^2
   = -\frac{qS}{V}\,\omega\left(S_2-2x_{cg}S_1+x_{cg}^2S_0\right)

となる。ここで :math:`S_n=\sum_i C_{N\alpha,i}\,x_i^n`\ （:eq:`eq-body-sums` の胴体の和にフィンの寄与を加えたもの）である。
重心は燃焼とともに移動するので、表には :math:`x_{cg}` に依存しない :math:`S_0,S_1,S_2` を Mach 数ごとに保存し、
飛翔計算のたびに\ :eq:`eq-damping` を組み立てる。括弧内が負になる場合（ボートテールの負の寄与が勝つ場合）は 0 とする。

括弧内の :math:`\max(0,\cdot)` を :math:`K` とおくと、無次元の微係数 :math:`C_{mq}=-2K/d^2` と書ける。
表には :math:`C_{mq}` を直接の列としては持たず、:math:`S_0,S_1,S_2` から作る。
この無次元の形、ヨーの :math:`C_{nr}`、各手法での出どころは :ref:`減衰とロールの微係数 <sec-aero-damping>` にまとめた。

同じ和は、回転による法線力（:eq:`eq-cnq-force`）にも使う。この力の重心まわりのモーメントが\ :eq:`eq-damping` である。

**遅れの和**

局所的な迎角の和は、断面積の変化 :math:`A'` による力だけを数える。
断面が横向きの運動に連れて動かす空気（見かけの質量）が、横向きの対気速度の変化と回転に遅れて付いてくる反作用は、
遅れの和 :math:`L_0` [m] と :math:`L_1` [m\ :sup:`2`] として別に表に持つ（飛翔計算での使い方は :ref:`sec-aero-unsteady`、表の列は :ref:`sec-aero-damping`）。
細長い物体の理論 :cite:`munk,nielsenbook` では、断面は単位長さあたり :math:`\rho A(x)` の空気を動かし、

.. math::
   :label: eq-lag-sums

   L_0=\frac{2}{S}\int A\,\mathrm{d}x,\qquad L_1=\frac{2}{S}\int A\,x\,\mathrm{d}x

である。:math:`A` は胴体の断面と、フィンのある断面の増分の和である。

* **胴体**：:math:`A=\pi r^2`。胴体全体では容積 :math:`\mathrm{Vol}` の空気を容積の中心 :math:`x_{\mathrm{vol}}` で動かすので、

  .. math::
     :label: eq-lag-sums-body

     L_0=\frac{2\,\mathrm{Vol}}{S},\qquad L_1=L_0\,x_{\mathrm{vol}}

  である。:math:`\mathrm{Vol}` と :math:`x_{\mathrm{vol}}` は、抽出した外形の点の間を円錐台で結んだ回転体（ノーズ先端の半径 0、後端は底面の半径）から求める。
  細長い物体の理論では Mach 数によらない。
  サンプルの胴体は :math:`L_0=2.722` m、:math:`L_1=2.226` m\ :sup:`2`\ （:math:`x_{\mathrm{vol}}=0.818` m）である。
* **フィン**：半径 :math:`r` の胴体に、軸からの半幅 :math:`s` のフィンが付いた断面の見かけの質量は :math:`\rho\pi(s^2-r^2+r^4/s^2)` で、平面形の 2 枚でも十字形の 4 枚でも同じである :cite:`nielsenbook`。
  胴体の :math:`\pi r^2` を除いた増分

  .. math::
     :label: eq-lag-section

     A_f(x)=\pi\left(s-\frac{r^2}{s}\right)^2

  をフィンの平面形の最も前の前縁から最も後ろの後縁（付け根か先端の後縁の後ろのほう）まで積分する。:math:`s` は前縁に沿って付け根の前縁の :math:`r` からフィン先端の前縁の :math:`r+\text{span}` まで増え、そこから後ろはその値のままとする（細長い物体の理論では、半幅が最大になった断面より後ろも同じ見かけの質量を持つ）。
  後退角が大きく先端の前縁が付け根の後縁より後ろにあるフィンでは、最大の半幅は付け根の後縁より後ろで達する。
  増分の大きさは、表の静的な法線力にそろえる。細長い物体の理論では最大の断面が法線力の傾き :math:`C_{N\alpha,f}=2A_{f,\max}/S` を決めるが、有限のアスペクト比のフィンではこの傾きは大きすぎるので、
  :math:`A_{f,\max}` を表のフィンの傾き（Barrowman の傾きに枚数と胴体との干渉を含めたもの。:eq:`eq-fin-total`）にあたる :math:`S\,C_{N\alpha,f}/2` とし、
  :math:`L_{0,f}=C_{N\alpha,f}\int\phi\,\mathrm{d}x`、:math:`L_{1,f}=C_{N\alpha,f}\int\phi\,x\,\mathrm{d}x`\ （:math:`\phi=A_f/A_{f,\max}`）とする。
  フィンの法線力の傾きとともに Mach 数で変わり、フィンの枚数の影響も傾きを通じて入る。
  サンプル（:math:`M=0.3`）では、細長い物体の理論のフィンの傾き :math:`2A_{f,\max}/S=14.23` が Barrowman の傾き 9.69 の 1.47 倍で、そろえたフィンの見かけの質量はそのぶん小さい。
  :math:`L_{0,f}=1.445` m、:math:`L_{1,f}=2.057` m\ :sup:`2`\ （中心 1.42 m）で、胴体と合わせて :math:`L_0=4.168` m、:math:`L_1=4.283` m\ :sup:`2` である。
* **吹き下ろしの遅れ**：後ろの揚力面が前の揚力面の後流の吹き下ろしを遅れて受ける項（航空機の水平尾翼の :math:`C_{m\dot\alpha}` :cite:`etkin`、カナードの後ろのフィン :cite:`nielsenbook`）は、フィンより前に揚力面があるときだけ生じる。
  部品積み上げ法が扱うフィンの組は 1 つで、その前に揚力面がないので、この項はない。見かけの質量とは別の効果である。

この遅れの和と\ :eq:`eq-damping` の局所的な迎角の和を合わせると、ピッチ振動の減衰（:math:`C_{mq}+C_{m\dot\alpha}`）は細長い物体の理論の閉じた式 :math:`-2C_{N\alpha}(l_e/d)^2`\ （:math:`l_e` は機体後端の重心からの距離）にサンプルで 6〜7 % 以内で一致する（:eq:`eq-sbt-damping`、:ref:`sec-verify-unsteady`）。
これは同じ Barrowman の傾きを使った組み立ての整合性の確認である。

パネル法と CFD の表、遅れの列のない外部の表にも、同じ形状から求めたこの値を入れる（:ref:`sec-file-v1`）。

.. _sec-barrowman-roll:

ロール（カントの強制と減衰）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

フィンにカント（取付角）を付けると、各フィンが少し迎角のついた翼になり、同じ向きの揚力が機軸まわりのモーメントを作って、機体は機軸まわりに回り出す（強制）。
回転が速くなると、フィンの各部は回転方向に動くので、気流に対する迎角が変わり、回転を妨げる揚力が生じる（減衰）。
強制とこの減衰が釣り合う角速度で、ロールは落ち着く。
強制をカントあたりのモーメント係数 :math:`C_{l\delta}`、減衰を無次元ロール角速度についての傾き :math:`C_{lp}` で表に持つ。
強制はフィン 1 枚の法線力の傾きから、減衰はフィンを機軸からの距離で切った細片の足し合わせ（ストリップ理論）から求める。

ロールのモーメント係数は :math:`C_l=M_x/(qSd)` で、強制項はカントについての傾き :math:`C_{l\delta}`\ [1/rad]、
減衰項は無次元のロール角速度 :math:`\hat p=pd/(2V)`\ （:math:`p` はロール角速度）についての傾き :math:`C_{lp}` で表す（:ref:`sec-roll-moment`）。

フィンの形状は :numref:`fig-fin-planform` の台形で、:math:`N` 枚のフィンは胴体半径 :math:`r` の位置から外側へ長さ :math:`s` だけ伸びる。
胴体の中心からの距離を :math:`\xi`\ （:math:`r\le\xi\le r+s`）とし、その位置の翼弦長を線形に変わる :math:`c(\xi)=a+(b-a)(\xi-r)/s` とする。

**カントによる強制項**

カント :math:`\delta` を付けると、各フィンの迎角が :math:`\delta` になる。1 枚の法線力は :math:`qS\,C_{N\alpha,1}\delta` で、
作用点のスパン方向の位置を平均空力翼弦の位置 :math:`y_{\mathrm{MAC}}=\dfrac{s}{3}\dfrac{a+2b}{a+b}` に置くと、機軸からの腕の長さは :math:`r+y_{\mathrm{MAC}}` である。
:math:`N` 枚のモーメントを :math:`qSd` で割って

.. math::
   :label: eq-cl-delta

   C_{l\delta}=\frac{N\,(r+y_{\mathrm{MAC}})\,C_{N\alpha,1}}{d}

となる :cite:`niskanen`。:math:`C_{N\alpha,1}` は基準面積 :math:`S` で無次元化した 1 枚のフィンの法線力傾斜である。
フィンの法線力と同じ式（亜音速の\ :eq:`eq-fin-sub`、超音速、遷音速の補間）から、干渉係数 :math:`K_{fb}` と枚数の係数 :math:`k_NN/2`\ （:eq:`eq-fin-total`）を除いた 1 枚分の値として求める。
このため :math:`C_{l\delta}` も Mach 数の関数で、遷音速で連続につながる。

**ロール減衰**

機体がロール角速度 :math:`p` で回ると、位置 :math:`\xi` のフィンの断片には、迎角が :math:`\Delta\alpha=p\,\xi/V` だけ加わる（ピッチ減衰の :math:`\omega\,\xi_i/V` と同じ考え方、:ref:`sec-damping`）。
断片の面積は :math:`c(\xi)\,\mathrm{d}\xi`、揚力傾斜を面積あたり :math:`a_f` とすると、揚力は :math:`qa_f\,\Delta\alpha\,c\,\mathrm{d}\xi`、機軸まわりのモーメントはその :math:`\xi` 倍である。
:math:`N` 枚を足して :math:`qSd` で割り、:math:`p/V=2\hat p/d` を使うと

.. math::
   :label: eq-clp-strip

   C_{lp}=-\frac{2\,a_f\,N}{S\,d^2}\int_r^{r+s}\xi^2\,c(\xi)\,\mathrm{d}\xi,\qquad
   \int_r^{r+s}\xi^2\,c(\xi)\,\mathrm{d}\xi=\left[\frac{(a-kr)\,\xi^3}{3}+\frac{k\,\xi^4}{4}\right]_r^{r+s},\quad k=\frac{b-a}{s}

となる（ストリップ理論）。符号は常に負で、回転を妨げる。フィンの枚数に比例する。

面積あたりの揚力傾斜 :math:`a_f` には、フィンの 3 次元の傾きを使う。1 枚のフィンの法線力傾斜は基準面積 :math:`S` で無次元化した :math:`C_{N\alpha,1}` なので、
フィン 1 枚の面積 :math:`A_f` あたりの傾きは

.. math::
   :label: eq-clp-slope

   a_f=\frac{C_{N\alpha,1}\,S}{A_f}

である。翼型の 2 次元の傾き（約 :math:`2\pi`）を使うと、アスペクト比の小さいフィンの端の揚力の低下（翼端損失）を無視するので、減衰を何倍も過大に見積もる。
フィン全体の傾きは有限スパンの効果を含むので、これを面積で割った :math:`a_f` を断片に使うほうが、有限スパンのフィンの減衰に近い値になる。

抗力（軸力）
~~~~~~~~~~~~

抗力は、表面摩擦（機体と翼の表面の粘性）、ノーズの圧力抗力（超音速では造波抗力）、底面抗力（後端の平らな面にかかる負圧）、フィンの前縁・後縁とボートテールの抗力に分けて求める。
亜音速では表面摩擦が大きく、遷音速から超音速では底面・フィンの前縁・ノーズの造波抗力が増える。
燃焼中は噴流が底面を埋めるので、燃焼中と燃焼後の 2 種類の軸力を持つ。

迎角 0 での抗力係数 :math:`C_{D0}` を、次の成分の和として求める :cite:`niskanen,hoerner`\ 。

.. math::
   :label: eq-cd0

   C_{D0} = C_{D,f} + C_{D,\mathrm{nose}} + C_{D,\mathrm{base}} + C_{D,\mathrm{fin}} + C_{D,\mathrm{bt}} + C_{D,\mathrm{extra}}

:math:`C_{D,\mathrm{extra}}` は、ラグやボタンなどの追加抗力として設定 ``rocket.extra_cd`` で与える定数である。

**表面摩擦**

Reynolds 数を全長 :math:`L` 基準で :math:`Re=VL/\nu` とする。
:math:`V` と動粘性係数 :math:`\nu` は射点高度の大気（設定した大気モデル）での値とし、:math:`V` は Mach 数と音速の積とする。
摩擦係数は、乱流の式と粗さで頭打ちになる式の大きい方をとる :cite:`niskanen,schlichting,hoerner`。

.. math::
   :label: eq-cf

   C_f^{\mathrm{turb}}=\frac{1}{(1.50\ln Re-5.6)^2},\qquad
   C_f^{\mathrm{rough}}=0.032\left(\frac{R_s}{L}\right)^{0.2},\qquad
   C_f=\max\!\left(C_f^{\mathrm{turb}},\,C_f^{\mathrm{rough}}\right)

ただし :math:`Re<10^4` では :math:`C_f=1.48\times10^{-2}` とする。:math:`R_s` は等価砂粒粗さ（設定 ``rocket.roughness``）で、0 のときは粗さの式を使わない。
圧縮性の補正 :cite:`niskanen` として

.. math::

   C_f^{\mathrm{turb}} \leftarrow
   \begin{cases} C_f^{\mathrm{turb}}(1-0.1M^2) & (M<1)\\ C_f^{\mathrm{turb}}/(1+0.15M^2)^{0.58} & (M\ge1)\end{cases},\qquad
   C_f^{\mathrm{rough}} \leftarrow
   \begin{cases} C_f^{\mathrm{rough}}(1-0.1M^2) & (M<1)\\ C_f^{\mathrm{rough}}/(1+0.18M^2) & (M\ge1)\end{cases}

を掛ける。:numref:`fig-skin-friction` に :math:`Re` と粗さによる変化を示す。サンプル機体（:math:`L=1.5` m、60 :math:`\mu\mathrm{m}`）では :math:`Re` が約 :math:`10^6` を超えると粗さの式が支配的になる。

.. _fig-skin-friction:

.. figure:: /_generated/plots/skin_friction.*
   :width: 80%

   表面摩擦係数。Reynolds 数が大きくなると、粗さで決まる一定値で頭打ちになる。見る点は、粗さが大きいほど頭打ちが低い Reynolds 数で始まり、値も大きいことである。

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

**底面抗力**

機体後端の平らな面には負圧がかかる :cite:`hoerner,niskanen`。底面積あたりの係数を

.. math::
   :label: eq-base

   C_{D,b}(M) = \begin{cases} 0.12+0.13M^2 & (M<1)\\ 0.25/M & (M\ge1)\end{cases}

とし、底面積 :math:`\pi r_{\mathrm{base}}^2` を掛ける。燃焼中は噴流が底面を埋めるので、ノズル出口面積 :math:`A_e` を差し引いた
:math:`\pi r_{\mathrm{base}}^2-A_e` を使う。このため係数表には燃焼中（power-on）と燃焼後（power-off）の 2 種類の :math:`C_A` がある。

**フィンの前縁・後縁**

丸めた前縁の抗力 :cite:`hoerner,niskanen`\ （前縁に垂直な成分、前縁の正面面積 :math:`Nst` あたり）は

.. math::

   C_{D,\mathrm{LE}\perp} =
   \begin{cases}
   (1-M^2)^{-0.417}-1 & (M<0.9)\\
   1-1.785\,(M-0.9) & (0.9\le M<1)\\
   1.214-\dfrac{0.502}{M^2}+\dfrac{0.1095}{M^4} & (M\ge1)
   \end{cases}

で、前縁の後退角 :math:`\Gamma_{LE}=\arctan(m/s)` について :math:`\cos^2\Gamma_{LE}` を掛ける。
3 つの式は :math:`M=0.9` と :math:`M=1` で連続につながる。前縁の項は設定 ``rocket.fin_le`` が ``"rounded"`` のときだけ加える。
後縁を切り落とした（square）フィン（``rocket.fin_te = "square"``）には、後縁の面積 :math:`Nst` に\ :eq:`eq-base` の底面抗力を加える。

**ボートテール**

ボートテールの長さ :math:`L_{bt}` と前後の直径差の比 :math:`\gamma=L_{bt}/(d_f-d_a)` によって

.. math::

   C_{D,\mathrm{bt}} = k_{bt}\,C_{D,b}(M)\,\frac{\pi(r_f^2-r_a^2)}{S},\qquad
   k_{bt} = \begin{cases} 1 & (\gamma<1)\\ (3-\gamma)/2 & (1\le\gamma\le3)\\ 0 & (\gamma>3)\end{cases}

とする。緩やかに絞ったボートテールほど抗力が小さい。

**抗力の内訳**

サンプル機体の抗力の内訳を :numref:`fig-aero-drag` に示す。
亜音速では表面摩擦が半分以上を占める。遷音速では底面抗力・フィン前縁の抗力・ノーズの造波抗力が増え、抗力係数は最大になる。

.. _fig-aero-drag:

.. figure:: /_generated/plots/aero_drag.*
   :width: 100%

   サンプル機体の零揚力抗力係数の内訳（燃焼後）。破線は燃焼中の合計。見る点は、亜音速で表面摩擦が最大の成分であり、遷音速で底面・フィンの縁・ノーズの成分が増えて合計が最大になることである。
   :math:`M=1` では、表面摩擦の圧縮性の補正の式が切り替わる（OpenRocket と同じ形。:eq:`eq-cf` の後の式）ので、表面摩擦が不連続に約 6 % 下がる。

**迎角による軸力の変化**

迎角があると軸力は変化する。OpenRocket と同じく :cite:`niskanen`、:math:`\alpha=17^\circ` で 1.3 倍の最大値をとり、:math:`90^\circ` で 0 になる
区分的な 3 次多項式（両端の傾き 0 の Hermite 補間）を掛ける（:numref:`fig-axial-factor`）。

.. math::
   :label: eq-ca-alpha

   C_A(M,\alpha) = C_{D0}(M)\,f(\alpha)

.. _fig-axial-factor:

.. figure:: /_generated/plots/axial_factor.*
   :width: 80%

   迎角による軸力係数の倍率 :math:`f(\alpha)`。見る点は、:math:`17^\circ` で最大の 1.3 倍になり、:math:`90^\circ` で 0 になることである。

適用範囲と精度
--------------

モデルの限界は次のとおりである。

* 経験式と細長体理論による近似で、小さな迎角の理論に、胴体揚力と大迎角のフィンの横流れの経験式を加えている。細長体理論は細長い軸対称の胴体と小さな迎角を前提にし、胴体揚力の係数 :math:`K` は経験値である。
* 遷音速（:math:`0.9<M<1.5`）の係数は補間で、物理モデルではない。
* フィンは台形の平板で、翼型の厚みや反りによる揚力の変化は含まない。フィン同士の干渉は枚数係数 :math:`k_N`、胴体との干渉は :math:`K_{fb}` で粗く表す。
* 胴体干渉係数 :math:`K_{fb}` はフィンが胴体に生む揚力の分を含まず、フィン部分の法線力が低めになる（サンプル機体で約 25 %）。静安定余裕は安全側だが、落下点はずれる。
* ピッチ減衰は準定常の近似で、フィンの揚力の履歴（循環の遅れ）と胴体の横流れの項を含まない。サンプルのフィンでは、循環の遅れがフィンの減衰を約 4 % 弱めると見積もる（:ref:`sec-quasi-steady`）。見かけの質量の項は、飛翔計算が迎角 20° から 30° で 0 に近づける（:eq:`eq-lag-fade`）。
* ノーズの抗力とロール減衰は、とくに不確かさが大きい。ロール減衰は、フィンどうしの干渉と胴体の影響を含まない。ロール角速度の予測は :math:`C_{l\delta}` と :math:`C_{lp}` の不確かさに強く依存するので、モンテカルロ解析の ``cl_scale`` と ``clp_scale``\ （:ref:`cfg-dispersion-monte_carlo-cl_scale`、:ref:`cfg-dispersion-monte_carlo-clp_scale`）で幅を見積もるとよい。
* 抗力の各成分は経験式である。表面摩擦は、射点高度の、設定した大気での Reynolds 数と、設定した粗さで決まる。

仮定と制約の全体は :ref:`sec-lim-aero` にある。

**検証**\ （式の実装を解析解・手計算と比べる）は :ref:`sec-vaero-barrowman` にある。

* 胴体の法線力傾斜が、ノーズの形によらず 2 になることを手計算と照合した。
* フィンの法線力と圧力中心が、亜音速・超音速・遷音速で連続につながることを確認した。
* カントを付けたサンプル機体の飛翔計算で、最大ロール角速度が、係数から見積もった値と一致した（:ref:`sec-vaero-barrowman-roll-flight`）。

**相互比較**\ （モデルどうし）では、ロール減衰が CFD の回転座標系の計算と同じ桁になる（:ref:`sec-vaero-barrowman-su2`）。
サンプル機体の :math:`C_{N\alpha}` は、Euler の CFD より亜音速で 26〜29 %、Mach 2 で 15 % 小さい（:ref:`sec-aero-accuracy-cross`）。

**実測との比較**\ は、Basic Finner の Mach 1.08 から 4.30 だけである（:ref:`sec-validation-results`）。

* :math:`C_{N\alpha}` は 20〜35 % 小さい。
* :math:`C_{X0}` は 23〜38 % 小さい。
* :math:`x_{cp}` は :math:`-2.4` から :math:`+2.4` % で合う。
* :math:`C_{lp}` は Mach 1.83〜3.15 で :math:`-8` から :math:`+5` %、低い超音速で過大、Mach 3.73 と 4.30 で 13〜15 % 小さい。

亜音速の実測との比較はない。差の大きさを、形の違う機体にそのまま当てはめることはできない（:ref:`sec-validation-limits`）。

選び方
------

部品積み上げ法は、形状を変えながら設計を進めるとき、経験式の範囲内の形状（軸対称の胴体、台形のフィン 1 組、標準的なノーズとボートテール）を扱うとき、ほかの手法と比べる基準がほしいときに向く。
仮定がパネル法や CFD と独立なので、差を不確かさの目安にできる。

次のときは選ばないほうがよい。

* 遷音速（Mach 0.9〜1.5）の抗力と安定が結論を左右する。この帯は物理を解かない補間である。
* フィンが大きい、胴体が太いなど、フィンと胴体の干渉が大きい。フィンの法線力が不足する（Basic Finner の実測に対して :math:`C_{N\alpha}` が 20〜35 % 小さい）。
* ポッドや非対称なフィンなど、軸対称の仮定が成り立たない。
* 絶対値が必要である。法線力と軸力が小さく出る向きに偏る（:ref:`sec-aero-accuracy-measured`）。静安定余裕は圧力中心が前に出るので安全側になるが、風見効果と落下点は変わる。

入力のうちどれを精密に決めるべきかは、:numref:`tbl-choose-barrowman-sens` の感度で判断する。
抗力の入力（``fin_le``、``fin_te``、``roughness``、``extra_cd``）は頂点と着地点を動かし、静安定余裕は動かさない。
フィンの寸法（``fin_override``）は法線力と圧力中心を通して余裕を動かす。カントはロールだけに効く。
基準の行だけは絶対値（頂点 [m]、着地距離 [m]、最小の静安定余裕 [cal]）で、ほかの行は基準からの差である。入力の欄の括弧内の数値は、迎角 0 の係数表の値の変化である（:math:`C_A` は燃焼後、:math:`x_{cp}` は m）。
差が小さい行（頂点で 1 % 未満、余裕で 0.1 cal 未満）の入力は、粗い見積もりで足りる。
``fin_override`` の行は、検出した寸法を丸めた値を基準に 1 項目だけ変えており、基準の行と ±0.6 m 以内で一致する。

.. _tbl-choose-barrowman-sens:

.. list-table:: 部品積み上げ法の入力を変えたときの結果の変化（サンプル機体、風 4 m/s（北）、パラシュート降下、``rk4``。``doc/data/sensitivity/barrowman_keys.csv``）
   :header-rows: 1
   :widths: 38 34 28

   * - 変えた入力（変わる項）
     - 頂点、着地距離、最小の静安定余裕
     - 判断
   * - 基準（サンプルの ``fin_le`` ``"rounded"``、``fin_te`` ``"square"``、``roughness`` 60 :math:`\mu`\ m、``extra_cd`` 0.02、``fin_cant_deg`` 0）
     - 4440.5 m、7735.5 m、1.99 cal
     - 
   * - ``fin_le`` ``"rounded"`` → ``"sharp"``\ （前縁の :math:`C_A` が Mach 0.3 で −0.005、Mach 2 で −0.136）
     - +458.3 m、+822.0 m、0.00 cal
     - 抗力の入力で頂点が最も動く。超音速で効くので、前縁の仕上げを実機に合わせる
   * - ``fin_te`` ``"square"`` → ``"tapered"``\ （底面抗力の :math:`C_A` が Mach 0.3 で −0.027、Mach 2 で −0.026）
     - +180.2 m、+321.7 m、0.00 cal
     - 後縁の切り落としの有無を実機に合わせる
   * - ``roughness`` 60 → 20 :math:`\mu`\ m（摩擦抗力の :math:`C_A` が Mach 0.3 で −0.061）
     - +247.8 m、+439.8 m、0.00 cal
     - 塗装の仕上げで頂点が数 % 動く。測れないときは幅を見る
   * - ``roughness`` 60 → 120 :math:`\mu`\ m（Mach 0.3 で +0.046）
     - −168.9 m、−301.2 m、0.00 cal
     - 同上
   * - ``extra_cd`` 0.02 → 0（:math:`C_A` に一定量 −0.020）
     - +91.1 m、+162.5 m、0.00 cal
     - ランチャラグなどの見積もりで頂点が約 90 m 動く
   * - ``extra_cd`` 0.02 → 0.04（+0.020）
     - −86.9 m、−155.2 m、0.00 cal
     - 同上
   * - ``fin_cant_deg`` 0 → 0.5°（ロールの :math:`C_{l\delta}` による強制）
     - −0.2 m、+0.2 m、0.00 cal
     - 頂点と着地はほぼ変わらない。ロールの量を知りたいときに決める
   * - ``fin_override.span`` 0.10 → 0.11 m（+10 %）（フィンの :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{l\delta}`、:math:`C_A`）
     - −111.6 m、−226.9 m、+0.27 cal
     - 余裕に最も効く。寸法は実測で入れる
   * - ``fin_override.span`` 0.10 → 0.09 m（−10 %）
     - +118.1 m、+248.1 m、−0.33 cal
     - 同上
   * - ``fin_override.count`` 4 → 3
     - +223.7 m、+464.9 m、−0.56 cal
     - 枚数は必ず合わせる
   * - ``fin_override.x_le_root`` 1.30 → 1.28 m（−20 mm）（Mach 0.3 で :math:`x_{cp}` が −0.016 m）
     - +0.5 m、+10.1 m、−0.16 cal
     - 余裕に直接効く。1 mm で約 0.008 cal

関連する設定
~~~~~~~~~~~~

``aero.method``\ （:ref:`cfg-aero-method`）、``rocket.fin_le``、``rocket.fin_te``\ （前縁・後縁の形）、``rocket.fin_cant_deg``\ （ロールの強制項 :math:`C_{l\delta}`）、``rocket.roughness``\ （表面摩擦）、``rocket.extra_cd``\ （ラグやボタンなどの追加抗力）、``aero.general_mach_step``\ （一般表にするときの Mach 数の刻み）。
キーの詳細は :ref:`cfg-rocket-fin_le` から始まる設定の節にある。
