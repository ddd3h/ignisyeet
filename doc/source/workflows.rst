.. _page-workflows:

計算の進め方
============

設定ファイルを自分の機体と当日の条件に合わせ、結果を得るまでの作業を、目的ごとに短い手順で示す。
最初にサンプルの計算が動いていること（:ref:`page-tutorial`）を前提とする。
手順は、機体、モータ、射点と回収系、環境、空力係数の表、条件を変えた繰り返し、落下分散、保安距離の順に進む。
設定キーの意味と範囲は :ref:`page-config` にあり、ここでは手順に必要なキーだけを挙げる。

.. _page-howto-own-rocket:

自分の機体を計算する
--------------------

自分の機体の CAD データと質量特性を設定ファイルに書き、``ignisyeet geom`` で形状の抽出結果を確かめる。
ここまでできれば、空力係数の計算と飛翔の計算に進める。形状の抽出が前提とするモデルと適用範囲は :ref:`page-theory-geometry` にある。

用意するものは次のとおりである。

* 機体の CAD データ。STEP、IGES、BREP のファイル（``rocket.cad``）か、外形を表す **閉じた表面** の STL（``rocket.stl``。ASCII でもバイナリでもよい）のどちらか一方を使う。CAD のファイルがあるときはそれを勧める。単位と形状の細部がファイルから得られるからである。STL は、メッシュしか書き出せないツールで機体を作ったときに使い、1 つのソリッドとして書き出す。
* 乾燥質量（推進剤を除く。空のモータケースを含む）、乾燥重心の位置、慣性モーメント。CAD で求めた値を使う。CAD がないときの見積もりは :ref:`sec-usage-inertia-estimate` にある。

1. 付属のサンプルをコピーし、自分の機体のファイルを置いて ``[rocket]`` の ``stl`` を書き換える（CAD のファイルを使うときは手順 2 の ``cad``）。相対パスは設定ファイルのあるディレクトリから解釈する。

   .. code-block:: sh

      cp ignisyeet-examples/sample.toml rocket.toml
      cp /path/to/my_rocket.stl .

   .. code-block:: toml

      [rocket]
      stl = "my_rocket.stl"

2. 単位を決める。STL は長さの単位を持たないので、m に直す係数 ``stl_scale`` を与える。mm で作った STL は ``0.001``、m なら ``1.0`` である（サンプルは ``0.001``）。
   手順 5 で表示される全長と直径が実機の寸法（m）と一致すればよい。

   機体の CAD ファイル（STEP、IGES、BREP）を使うときは、STL に書き出さず、``stl`` の代わりに ``cad`` を書く。
   CAD の長さの単位はファイルが宣言する値で読むので、``stl_scale`` は書かない（書くとエラーになる）。単位を宣言しない STEP と IGES、または宣言と違う単位で読みたいときは、``cad_unit``\ （``"mm"`` など）で指定する。BREP は単位を持たず m として読む。
   形状の抽出以降（空力係数の表と飛翔の計算）は STL の入力と同じ手順で進む。CAD ファイルを読むには CFD 用ツールの gmsh（Python）が要る（導入は :ref:`page-howto-install`）。

   .. code-block:: toml

      [rocket]
      cad = "my_rocket.step"
      # cad_unit = "mm"

3. ノーズの向きを確かめる。``nose_direction`` の既定 ``"auto"`` は形状から向きを決める。判定が怪しいと手順 5 で警告が出る（:ref:`IY-G503`、:ref:`IY-G504`、:ref:`IY-G509`）。
   そのときは入力の座標でノーズが向く方向を ``"+x"`` ``"-x"`` ``"+y"`` ``"-y"`` ``"+z"`` ``"-z"`` のいずれかで指定する。

4. 質量特性を書く。機体上の位置はすべてノーズ先端から後方へ測る。

   .. code-block:: toml

      dry_mass = 8.0      # kg。空のモータケースを含み、推進剤は含まない
      cg_dry = 0.85       # m。乾燥重心の位置
      ixx_dry = 0.02      # kg m^2。ロール慣性モーメント（乾燥重心まわり）
      iyy_dry = 1.5       # kg m^2。ピッチ（izz_dry を省くとヨーも同じ値）

   重心が機軸からずれるとき、ヨーとピッチの慣性モーメントが異なるとき、慣性乗積があるときは、``cg_lateral_dry``、``izz_dry``、``ixy_dry`` などを続けて書く。
   慣性乗積の符号は CAD の表示と逆になることが多い（:ref:`sec-config-rocket`）。

5. 形状を抽出して確かめる。

   .. code-block:: sh

      ignisyeet geom rocket.toml

   サンプルでは次のように表示される。

   .. code-block:: text

      Geometry
        length          1.5000 m
        diameter        0.1000 m (ref area 0.007854 m^2)
        nose            length 0.2913 m, joint angle 1.47 deg, volume ratio 0.529
        fins            4 x root 0.1991 tip 0.0998 span 0.1000 sweep 0.0799 t 0.0040 at x 1.3000 m
        -> out/geometry.json
        -> out/profile.csv

   全長、直径、フィンの枚数と寸法が実機と合い、ノーズの長さが直径の 0.4 倍から 8 倍に収まる。
   ボートテールのある機体では ``boattail`` の行も出る。

6. 抽出した断面を図で確かめる。

   .. code-block:: sh

      ignisyeet-plot geometry out

   ``out/plots/geometry.png`` ができる（:numref:`fig-howto-geometry-profile`）。見るのは、実線の抽出結果が機体の外形に重なっていることである。

   .. _fig-howto-geometry-profile:

   .. figure:: /_generated/plots/geometry_profile.*
      :width: 100%
      :alt: サンプル機体の抽出結果

      サンプル機体で抽出した胴体半径とフィン。破線は設計値で、見るのは抽出結果（実線）が設計値に重なっていることである。

   フィンの検出が複雑な形状（エアブレーキ、カメラポッド、非対称な突起など）で外れたときは、``[rocket.fin_override]`` に正しい値を書く。書いた値が検出結果より優先され、必要な項目だけでよい。

   .. code-block:: toml

      [rocket.fin_override]
      count = 4
      root_chord = 0.20
      tip_chord = 0.10
      span = 0.10
      sweep = 0.08
      thickness = 0.004
      x_le_root = 1.30

7. 警告を確かめる。形状に不自然な点があると、``geom`` ``aero`` ``sim`` の実行時に警告が出て、``geometry.json`` の ``warnings`` にも残る。疑わしい点がなければ ``warnings`` は空である。

.. _sec-geometry-warnings:

抽出結果の警告
~~~~~~~~~~~~~~

警告の症状と見るコードは次のとおりである。しきい値と原因は各コードの項にある。

.. list-table::
   :header-rows: 1
   :widths: 60 40

   * - 症状
     - コード
   * - 全長が直径に比べて極端に短いか長い（``stl_scale`` の誤り）
     - :ref:`IY-G505`
   * - ノーズの長さが不自然（向きか検出の誤り）
     - :ref:`IY-G506`
   * - 底面が胴体より極端に小さい（突き出したノズル、強いボートテール）
     - :ref:`IY-G508`
   * - フィンが胴体の底面より後ろへ突き出している
     - :ref:`IY-G507`、:ref:`IY-G512`
   * - フィンが前方にある（機体が逆向きか、位置の誤り）
     - :ref:`IY-G509`、:ref:`IY-G510`
   * - フィンのスパンが大きすぎる
     - :ref:`IY-G511`
   * - 短い突起を基準半径の決定から除いた
     - :ref:`IY-G502`
   * - ノーズの向きを端の半径だけで決めた、または手がかりが食い違う
     - :ref:`IY-G504`、:ref:`IY-G503`
   * - 表面に小さなメッシュの欠陥がある
     - :ref:`IY-G501`
   * - 表面が閉じていない（エラー。穴をふさぐか、ソリッドとして書き出し直す）
     - :ref:`IY-G010`、:ref:`IY-G011`
   * - 機体の大きさが尺度の誤りを疑わせる、乾燥重心やモータが機体に収まらない
     - :ref:`IY-O005`、:ref:`IY-O006`

サンプルの STL（mm）に ``stl_scale = 1.0`` を指定すると全長が 1500 m になり、``error[IY-O005]`` で止まる（メッセージの末尾に、mm を m に直す値 ``0.001`` が出る）。
サンプルに ``nose_direction = "-z"`` を指定すると、ノーズが逆向きに読まれて ``warning[IY-G506]``、``[IY-G508]``、``[IY-G509]`` が同時に出る。

``out/geometry.json`` の ``body_x`` ``body_y`` ``body_z`` は、機体座標の軸の向きを入力の座標で表した単位ベクトルである。慣性乗積や ``cg_lateral_dry`` の符号は、この向きで決める（:ref:`sec-body-basis`）。

.. _page-howto-motor:

モータを設定する
----------------

モータの推力曲線を RASP 形式（``.eng``）のファイルで用意し、``[motor]`` に書く。
推力曲線は燃焼試験の実測値か、市販モータの公開データ（ThrustCurve.org など）を使う。IgnisYeet は推力を計算せず、ファイルの推力履歴をそのまま使う（:ref:`page-theory-propulsion`）。

1. 推力曲線を RASP 形式で書く。コメント行（``;`` で始まる）、ヘッダ 1 行、「時刻 [s] と推力 [N]」の組の行の順である。付属のサンプルモータ（架空）は次のとおりである。

   .. code-block:: text

      ; Fictional motor for the IgnisYeet example (not a real product).
      ; name  dia[mm] len[mm] delays prop[kg] total[kg] maker
      IY-SAMPLE 75 500 0 3.0 4.5 IgnisYeet
      0.00 0
      0.05 2300
      0.20 2400
      1.00 2300
      2.00 2150
      2.60 2000
      2.90 900
      3.10 0

   ヘッダは、名前、直径 [mm]、長さ [mm]、遅延、推進剤質量 [kg]、全備質量 [kg]、製造者の 7 項目である。
   使うのは直径、長さ、推進剤質量で、全備質量は検査するだけである。したがって空のモータケースの質量は ``rocket.dry_mass`` に含める。
   時刻の間は線形に補間し、最後の時刻を燃焼終了とする。先頭が :math:`t=0` でないときは :math:`(0,0)` を補う。

2. ``[motor]`` を書く。``eng`` は必須で、相対パスは設定ファイルのあるディレクトリから解釈する。

   .. code-block:: toml

      [motor]
      eng = "sample_motor.eng"
      # aft_x = 1.5                  # モータ後端の位置 [m]（ノーズ先端から後方へ）。省くと機体の全長
      nozzle_exit_diameter = 0.03    # ノズル出口の直径 [m]。燃焼中はこの面積だけ底面抗力が減る

   モータが機体の後端に収まっているなら ``aft_x`` は省ける。機体の途中に置くときは後端の位置を書く。

3. ヘッダの推進剤質量が実測と異なるときだけ、``propellant_mass`` で上書きする。初期質量は ``dry_mass + propellant_mass`` になる。

4. 推力線のずれを再現するときは、``misalignment_deg``、``misalignment_azimuth_deg``、``misalignment_offset`` を書く（既定は 0）。

5. 飛翔を計算し、燃焼時間を確かめる。

   .. code-block:: sh

      ignisyeet sim rocket.toml
      grep burnout_time out/summary.json

   サンプルでは ``"burnout_time": 3.1,`` と出て、``.eng`` の最後の時刻と一致する。結果パネルの ``rail exit`` の行にはランチャ離脱の時刻と速度が出る。

.. _sec-propulsion-eng:

``.eng`` の検査
~~~~~~~~~~~~~~~

読み込み時に ``.eng`` を検査し、誤りがあればファイル名と行番号を示してエラーにする。疑わしいだけの推力曲線は警告になる。

.. list-table::
   :header-rows: 1
   :widths: 60 40

   * - 症状
     - コード
   * - 見出し行の項目が足りない、数値でない、直径・長さ・質量が不正
     - :ref:`IY-S002` から :ref:`IY-S009`
   * - 時刻や推力が不正、時刻が増加していない、全力積が正でない
     - :ref:`IY-S010` から :ref:`IY-S015`
   * - 推力曲線が 1 点だけ
     - :ref:`IY-S500`
   * - 最後の点の推力が 0 N でない
     - :ref:`IY-S501`
   * - 燃焼時間が極端に短い
     - :ref:`IY-S502`

時刻が増加していないと ``error[IY-S013]: cannot read motor file …`` の下に ``line 6: thrust curve times must strictly increase, got 0 after 0.05`` のように行番号と原因が出る。
レールを離れずに滑り落ちたときの ``error[IY-S080]`` では、推力曲線の単位（N）、機体の質量、``launch.rail_length`` を確かめる。
モータが機体より長い、または機体の外にあるときの ``error[IY-O007]``、``[IY-O008]`` では、``.eng`` のヘッダの長さ [mm]、``rocket.stl_scale``、``motor.aft_x`` を確かめる。

.. _page-howto-launcher-parachute:

射点とパラシュートを設定する
----------------------------

射点、射角、レール長、パラシュート（単一または二段開傘）を設定し、結果で離脱速度と開傘の速さを確かめる。
降下方式の選び方と開傘のモデルは :ref:`page-theory-descent`、ランチャ滑走の運動方程式は :ref:`page-theory-dynamics` にある。
用意するものは、射場の緯度・経度・標高、ランチャのレール長、パラシュートの抗力面積 :math:`C_DS` である。

1. 射点を書く。緯度・経度は WGS84 の度、標高は海抜 [m] である。KML で地形に重ねるとき、標高が地形の高さと合っていないと経路が地面にめり込んだり浮いたりして見える（:ref:`sec-kml`）。

   .. code-block:: toml

      [launch]
      latitude = 34.7347
      longitude = 139.4204
      altitude = 20.0            # 射点の海抜標高 [m]

2. 射角と射方位を書く。射方位は傾けた機体のノーズが向く水平方位であり、風向（風が吹いてくる方位）とは別の量である。範囲外の値（射角が 0 以下または 90 を超えるなど）はエラーで止まる。

   .. code-block:: toml

      elevation_deg = 85.0       # 水平面からの仰角 [deg]
      azimuth_deg = 270.0        # 射方位。北から時計回り [deg]（270 は西向き）
      # roll_deg = 0.0           # ランチャ上の機体のロール角 [deg]。0 では y_b が水平

3. レール長を書く。機体が発射位置からレールに沿って進み、レールを離れるまでの有効な案内長である。
   後端のラグがレールの端にないときは、ラグが発射位置からレールの端まで動ける距離を入れる（:ref:`sec-rail`）。

   .. code-block:: toml

      rail_length = 5.0          # [m]

   ``rail exit`` の行に離脱の時刻と速度と静安定余裕が出る。サンプル（5 m）では ``0.25 s, 45.7 m/s, stability 1.99 cal`` である。

4. 離脱速度と風を確かめる。離脱速度は 15 m/s 以上、風による迎角は 15° 以下を目安にする。外れると警告が出る。

   * ``rail exit speed … m/s is below 15 m/s``\ （:ref:`IY-O515`）：機体がレールの線から風で流されやすい。レールを長くするか推力重量比を上げる。
   * ``the wind angle of attack at rail exit is … deg``\ （:ref:`IY-O516`）：離脱の高さの風と離脱速度から求めた迎角が 15° を超える。

   サンプルでレール長を変えたときの離脱速度は次のとおりである（部品積み上げ法、風 4 m/s、射角 85°）。

   .. list-table::
      :header-rows: 1
      :widths: 30 35 35

      * - ``launch.rail_length``
        - 離脱の時刻と速度
        - 警告
      * - 5.0 m（サンプル）
        - 0.25 s、45.7 m/s
        - なし
      * - 1.0 m
        - 0.12 s、20.0 m/s
        - なし
      * - 0.5 m
        - 0.10 s、13.9 m/s
        - :ref:`IY-O515`
      * - 0.3 m
        - 0.08 s、10.6 m/s
        - :ref:`IY-O515`

   風速 12 m/s、レール長 2 m にすると、離脱速度は 28.5 m/s、風による迎角は 22.8° になり :ref:`IY-O516` が出る。

5. 単一のパラシュートを設定する。``delay`` は頂点から開傘までの時間である。頂点 + ``delay`` にモータがまだ燃えているときは、開傘を燃焼終了まで遅らせて警告する（:ref:`IY-O517`）。
   ``enabled = false`` のときは、落下分散を含めて弾道落下だけを計算する。

   .. code-block:: toml

      [recovery]
      enabled = true
      cd_s = 3.5                 # 抗力面積 C_D S [m^2]
      delay = 1.0                # 頂点から開傘までの遅れ [s]

   設定パネルに終端降下速度の見積り（``terminal speed``）が出る。サンプル（乾燥質量 8 kg、:math:`C_DS = 3.5\ \mathrm{m^2}`）では 6.0 m/s である。
   これは海面の標準密度と乾燥質量から求めた見積りで、着地速度（横方向の速度を含む。サンプルでは 7.1 m/s）とは一致しない。

6. 二段開傘にするときは、``main_cd_s`` と ``main_altitude`` を **両方** 書く（片方だけはエラー。:ref:`IY-S074`）。

   .. code-block:: toml

      [recovery]
      enabled = true
      cd_s = 0.5                 # ドローグ
      delay = 1.0
      main_cd_s = 8.0            # メイン
      main_altitude = 300.0      # メインの開傘高度 [m]（射点からの高さ）

   ``cd_s`` と ``delay`` のパラシュートがドローグになる。射点からの高さが ``main_altitude`` に達すると、メインが開き、抗力面積が ``main_cd_s`` に **置き換わる**\ （足し合わせない）。
   メインの高度が頂点より高いときは、ドローグと同時にメインが開く（:ref:`IY-O511`）。
   サンプルの ``cd_s`` を 0.5、メインを 8 m\ :sup:`2`、300 m にした結果は次のとおりである（部品積み上げ法、風 4 m/s）。

   .. code-block:: text

      deploy          27.10 s, 28.5 m/s, q 0.32 kPa
      main deploy     300.0 m at 257.87 s, 16.3 m/s, q 0.16 kPa
      landing         331.9 s, E -681.7 m, N -3106.4 m (34.706698, 139.412959), 4.8 m/s

   単一のパラシュート（サンプル）では着地が 685.0 s 後で北 −7707.7 m、二段開傘では 331.9 s 後で北 −3106.4 m と、ドローグで速く降りた分だけ流される距離が短くなる。

7. 降下方式は ``sim`` の ``--descent`` で、設定を書き換えずに選べる。省くと、``recovery.enabled`` が真ならパラシュート、偽なら弾道落下になる。

   .. code-block:: sh

      ignisyeet sim sample.toml --descent ballistic
      ignisyeet sim sample.toml --descent parachute

   サンプルの弾道落下では、着地が 61.9 s 後、着地点が東 −1072.9 m、北 372.3 m で、着地速度は 178.2 m/s である。
   回収系が無効なのに ``--descent parachute`` を指定すると ``error[IY-O001]`` になる。結果パネルの見出し（``Flight (Parachute, …)`` または ``Flight (Ballistic, …)``）で、選んだ方式を確かめる。

結果パネルまたは ``summary.json`` で次を確かめる。

* ランチャ離脱速度（``rail_exit_speed``）が 15 m/s 以上である。
* 開傘時の対気速度が 50 m/s 以下、動圧が 2 kPa 以下である（``drogue_deploy_speed``、``drogue_deploy_q``。二段開傘ではメインの値も）。超えるとパラシュートが壊れるおそれがあるとして警告する（:ref:`IY-O518`）。サンプルの風を 12 m/s にすると、対気速度は 60 m/s、動圧は 1.5 kPa になり警告が出る。
* 着地速度（``landing_speed``）が 15 m/s 以下である。超えると警告し（:ref:`IY-O520`）、着地までに開傘しなかったときも警告する（:ref:`IY-O519`）。
* ``sim.max_time`` までに着地しないと ``warning[IY-O504]`` と ``INCOMPLETE RESULTS`` が出る。降下方式とパラシュートの設定を確かめるか、``sim.max_time`` を大きくする。

.. _page-howto-environment:

風・大気・地球を設定する
------------------------

打上げ当日の風を計算に入れる方法として、地上風だけを与える方法と、高度別の風の表（CSV）を使う方法を示す。続けて、表の風全体を変える方法と、地球と大気のモデルの指定を示す。
どのモデルを選ぶかの判断は :ref:`page-environment` にある。用意するものは風の観測値または予報で、地上風だけなら風速と風向（風が **吹いてくる** 方位）で足りる。

1. 地上風だけで計算する（``constant``、``power``、``log``）。``speed`` は基準高度 ``ref_height``\ （既定 2 m）での風速で、``power`` と ``log`` はこれを上空へ外挿する。

   .. code-block:: toml

      [wind]
      model = "power"        # "constant" | "power" | "log"
      speed = 4.0            # 基準高度での風速 [m/s]
      direction_deg = 0.0    # 風が吹いてくる方位。北から時計回り [deg]（270 は西風）
      ref_height = 2.0       # 基準高度 [m]

   結果パネルの見出しの下に ``wind 4.0 m/s from 0 deg`` と出る。
   頂点が 1 km を超える飛翔では、上空の風が外挿に依存するため警告する（:ref:`IY-O514`）。サンプルは頂点が 4440 m なので毎回出る。

2. 高度別の風の表を作る。予報（GFS、ECMWF など）やラジオゾンデの風は CSV にして使う。
   1 行目は次の見出しでなければならない（大文字小文字と前後の空白は無視する）。空行と ``#`` で始まる行は読み飛ばす。

   .. code-block:: text

      altitude_m,speed,direction_deg
      0,3.0,270
      100,6.0,270
      300,9.0,285
      600,12.0,300
      1000,13.0,310

   各列は、射点からの高度 [m]（0 以上で厳密に増加）、風速 [m/s]（0 以上）、風が吹いてくる方位 [deg]（0 以上 360 以下）である。
   予報やゾンデのデータは次のように換算する。

   * 高度が海抜なら、``launch.altitude`` を引いて射点からの高さにする。負になる行は捨て、0 m の行を加える。その行には、捨てた行のうち射点に最も近い風か、射点の地上風を入れる。
   * 気圧面ごとのデータは、同じ製品のジオポテンシャル高度を高度の列に使う。
   * 風速が knot なら 0.5144 を掛けて m/s にする。
   * 東西成分 :math:`u`\ （東向き正）と南北成分 :math:`v`\ （北向き正）なら、風速は :math:`\sqrt{u^2+v^2}`、風が吹いてくる方位は :math:`\mathrm{atan2}(-u,\,-v)`\ [deg] を 0° 以上 360° 未満にした値である（西風は 270°）。
   * 表は予想される頂点より上まで続ける。上端より上は上端の風で一定になり、足りないと警告する（:ref:`IY-O512`）。表の最下行が 0 m より上でも警告する（:ref:`IY-O513`）。

   付属の ``wind_profile_example.csv`` は書式を示す架空の表で、実測値ではない。

3. 設定ファイルで表を指定する。``profile`` では ``speed`` と ``direction_deg`` を使わない。サンプルのようにこの 2 つが書いてあると、使わないことを知らせる警告が出る（:ref:`IY-C502`、:ref:`IY-C503`）ので、自分の設定ファイルでは消しておく。

   .. code-block:: toml

      [wind]
      model = "profile"
      profile = "wind_profile_example.csv"   # 設定ファイルのあるディレクトリからの相対パス

   設定ファイルを書き換えずに試すときは、``--set`` を使う（サンプルでは警告が毎回出る）。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.model=profile --set wind.profile=wind_profile_example.csv

   サンプルでは次の行が出る。

   .. code-block:: text

      warning[IY-C502]: wind.speed is ignored for wind.model = "profile"; use wind.profile_mean_speed to scale the profile
      warning[IY-C503]: wind.direction_deg is ignored for wind.model = "profile"; use wind.profile_mean_direction_deg to rotate the profile
      Flight (Parachute, wind 11.9 m/s from 220 deg)
      ...
        apogee          4478.3 m at 26.22 s
      ...
        landing         690.1 s, E -272.4 m, N 4384.1 m (34.774219, 139.417425), 6.8 m/s
        profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)

   ``Flight`` の行の風は表全体の代表風である。代表風速 :math:`W` は表の最下行から最上行までの平均風速、代表風向 :math:`\Psi` は同じ範囲で平均した風速ベクトルが吹いてくる方位で、``summary.json`` の ``wind_speed`` と ``wind_direction_deg`` にも残る。
   最後の行は、表を ``wind.ref_height`` での風に直した値である。

4. 表の形（高度ごとの風速の比と風向のねじれ）を保ったまま代表風だけを変えるには、``profile_mean_speed`` と ``profile_mean_direction_deg`` を書く。風速はすべて同じ比で拡大・縮小され、風向はすべて同じ角度だけ回る。片方だけを指定すると、もう片方は表のままである。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.model=profile --set wind.profile=wind_profile_example.csv \
          --set wind.profile_mean_speed=15.6 --set wind.profile_mean_direction_deg=270

   ``Flight`` の行が ``wind 15.6 m/s from 270 deg`` になる。サンプルでは頂点 4495.4 m、着地点が東 4101.2 m、北 3306.8 m である（部品積み上げ法）。
   ほぼ無風の表（平均風速 0.1 m/s 未満）は拡大・回転できずエラーになる（:ref:`IY-S050`）。拡大率が 3 を超える、または 1/3 を下回ると、上空の風が非現実的になるため警告する（:ref:`IY-O521`）。

5. 地球と大気を設定する。

   .. code-block:: toml

      [earth]
      model = "flat"             # "flat" | "ecef"
      gravity = "inverse_square" # "constant" | "inverse_square" | "j2"

      [atmosphere]
      model = "us1976"           # "us1976" | "constant"
      temperature_offset = 0.0   # 標準大気の温度に足す差 [K]（us1976 のみ）

   ``flat`` は射点の接平面の座標で、``ecef`` は WGS84 楕円体に固定した回転座標系（コリオリ力と遠心力を含む）で計算する。``gravity = "j2"`` は ``ecef`` のときだけ指定できる（:ref:`IY-S054`）。
   当日の気温が標準と違うときは ``temperature_offset`` で差を与える。``constant`` の大気では ``density``、``sound_speed``、``viscosity`` を与える（すべて正）。

表の読み込みに失敗すると ``error[IY-S034]: cannot load wind profile …`` となり、``Caused by:`` の最後の行に原因と該当の行が出る。
たとえば風向が 400 だと ``wind profile direction must be between 0 and 360 degrees (row "100,6.0,400")``、高度が増えていないと ``wind profile altitudes must be strictly ascending (row "0,6.0,270")``\ （コードは :ref:`IY-S036`）と出る。
見出し、行の形、値の範囲、高度の順序の誤りは別々のコード（:ref:`IY-S030` から :ref:`IY-S037`）になる。``model = "profile"`` で ``profile`` を書き忘れると :ref:`IY-S045` になる。

.. _page-howto-external-table:
.. _sec-usage-external-table:

外部の係数表を使う
------------------

風洞試験や他の解析で得た空力係数を、``aero.method = "table"`` で飛翔計算に使う。
用意するものは、係数表の CSV（列の名前と順序は、このソフトが書く ``aero_table.csv`` と同じ。:ref:`sec-aero-table-format`）と、表の基準量（基準面積、基準直径、全長）および格子（Mach 数、迎角）である。
機体の質量・重心・慣性とモータは通常どおり ``[rocket]`` と ``[motor]`` で与える。係数表が置き換えるのは空力係数だけである。
表の考え方、内挿と外挿、格子の刻みは :ref:`page-theory-aero-tables` にある。

1. ``out/aero_table.csv`` を手本にして ``ext_table.csv`` を作る。見出し行と同じ列を、Mach 数を外側、迎角を内側にして全格子点を 1 行ずつ並べる。
   一般表（迎角と横滑り角の 2 次元）では横滑り角を最も内側にする。列の定義は :ref:`sec-file-axisym` と :ref:`sec-file-general` にある。

2. CSV と同じ名前で拡張子だけ ``.json`` にした付随ファイル（``ext_table.json``）を同じディレクトリに置く。
   必須の項目は ``source_hash``\ （外部の表では任意の文字列。表の出所を表す名前など）、``ref_area``、``ref_diameter``、``length``、``machs``、``alphas_deg``、``extrapolation`` である（:ref:`sec-file-sidecar`）。
   一般表では ``kind = "general"`` と ``betas_deg`` を加える。格子は CSV と一致させる。各軸の点数の積が CSV の行数になる。

3. ``[aero]`` に 2 行書く。``table`` の相対パスは設定ファイルのあるディレクトリから解決される。機体の形状の抽出と空力モデルの計算は行われず、表がそのまま使われる。

   .. code-block:: toml

      [aero]
      method = "table"
      table  = "ext_table.csv"

4. ``sim`` を実行する。

   .. code-block:: console

      $ ignisyeet sim sample.toml

   先頭近くに ``Using external aero table ext_table.csv (13741 rows)`` の形の行が出て、飛翔の結果が続く。
   この行数は、``examples/out/aero_table.csv``\ （Mach 0 から 3、刻み 0.02、迎角 0 から 90 deg、刻み 1 deg）を付随ファイルとともに外部の表として読ませたときの値で、飛翔の結果は同じ機体を ``method = "barrowman"`` で計算したときと一致する（apogee 4440.5 m、着地 E -655.6 m、N -7707.7 m）。

5. 軸対称の外部表を横滑りの連成を含む計算に使うときだけ、``aero.table_kind = "general"`` を加える。外部の表が最初から一般表なら何も加えなくてよい。

確認は ``out/summary.json`` の ``aero_table_range`` で行う。``machs_in_range`` と ``angles_in_range`` がともに ``true`` なら、飛翔中の全引き出しが表の格子の中に収まっている。
上の例では ``lookups`` が 54209 回、``max_abs_alpha_deg`` が 5.82 deg、``max_mach`` が 1.54 で、範囲外は 0 である。
表の範囲を外れると、``sim`` と ``dispersion`` は ``RESULTS NOT USABLE: [IY-O024] …`` を書き、終了ステータス 3 で終わる（:ref:`IY-O024`、:ref:`sec-exit-status`）。迎角の上限を 90 deg 以上にして表を作り直す。軸対称の表では範囲外の引き出しは起きない。
ロール項（``cl_delta`` と ``clp``）がすべて 0 の表は、機体の入力から抽出したフィンの寸法で補われ、端末に注意書きが出る。ロール項を含む表はそのまま使われる。
表の値の定義（座標系、モーメントの基準点、減衰の和）は :ref:`sec-aero-table-format-external` に従う。

読み込みの誤りは次のコードで報告される。

.. list-table::
   :header-rows: 1
   :widths: 46 54

   * - 症状
     - 対処
   * - ``error[IY-C047]``
     - ``[aero]`` に ``table`` を書く
   * - ``error[IY-A041]``、``error[IY-A042]``
     - CSV と同じ名前の ``.json`` を置く。JSON の書式と項目の型を直す
   * - ``error[IY-A043]``
     - ``aero.table`` のパスと読み取り権限を確かめる
   * - ``error[IY-A044]``
     - 見出し行の列名と順序を手本と一致させる
   * - ``error[IY-A045]`` から ``error[IY-A047]``
     - 数値でない、値の数が見出しと違う、NaN か無限大がある。メッセージの行番号の行を直す
   * - ``error[IY-A048]`` から ``error[IY-A050]``
     - 行の座標が付随ファイルの格子と一致しない、または行数が合わない
   * - 終了ステータス 3、``[IY-O024]``
     - 表の迎角の上限を 90 deg 以上にして作り直す

.. _page-howto-sweeps:

設定を変えて計算を繰り返す
--------------------------

設定ファイルを書き換えずに、コマンドラインで風や空力の手法だけを変えて計算を繰り返す。パネル法や CFD の係数表を一度だけ作って使い回す方法も示す。
オプションの完全な規則は :ref:`page-reference-cli` にある。

1. ``--set KEY=VALUE`` で設定を 1 つ変える。``KEY`` は設定ファイルの項目を ``.`` でつないだもの、``VALUE`` は TOML の値で、何度でも指定できる。設定ファイルは書き換わらない。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.speed=7 --set wind.direction_deg=90

   ``Flight`` の行が ``wind 7.0 m/s from 90 deg`` になる。

   * 文字列の値は ``wind.model=profile`` のように引用符なしで書ける。
   * 同じキーを 2 回指定すると最後の値を使い、警告する（:ref:`IY-C500`）。
   * 値の型や名前を誤ると ``error[IY-C005]: invalid config … with the --set overrides`` で止まり、原因が ``--set wind.speed=abc: invalid type: string "abc", expected f64`` の形で出る。``KEY=VALUE`` の形でない引数は終了ステータス 2 で止まる（:ref:`IY-C002`）。
   * ``--set`` は項目を消せない。省略した場合と同じにしたい項目（二段開傘の ``recovery.main_cd_s`` と ``main_altitude`` など）は、元の設定ファイルに書かず、必要な計算でだけ ``--set`` で加える。

2. ``--out DIR`` で出力先を分ける。指定しないとすべての計算が同じ ``out/`` に書き、前の結果を上書きする。相対パスは設定ファイルのディレクトリではなく **現在のディレクトリ** から解釈する。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.speed=7 --set wind.direction_deg=90 --out runs/wind_7ms

3. 風速を変えた計算を繰り返す。風速 1 から 7 m/s の東風で ``sim`` を 7 回実行し、着地距離を並べる。
   ``-q`` は標準出力にファイルのパスだけを出し、エラーと警告を標準エラー出力に出す。``> /dev/null`` でパスを捨てると警告だけが画面に残る（サンプルでは毎回 :ref:`IY-O514` が出る）。

   .. code-block:: bash

      for v in 1 2 3 4 5 6 7; do
        ignisyeet sim sample.toml -q --set wind.speed=$v --set wind.direction_deg=90 --out runs/w${v}ms > /dev/null
        echo "$v m/s: $(grep -h '"landing_distance"' runs/w${v}ms/summary.json)"
      done

   結果は次のとおりである（部品積み上げ法、サンプルの機体と射点。小数の下位の桁は環境で変わることがある）。

   .. code-block:: text

      1 m/s:   "landing_distance": 2614.0317273920805,
      2 m/s:   "landing_distance": 4572.92207030218,
      3 m/s:   "landing_distance": 6535.273304303107,
      4 m/s:   "landing_distance": 8496.338756128222,
      5 m/s:   "landing_distance": 10451.438904325232,
      6 m/s:   "landing_distance": 12392.931460473937,
      7 m/s:   "landing_distance": 14315.553970590654,

   風の条件を網羅するときは、``ignisyeet dispersion`` が風速 × 風向の格子を並列に計算する（:ref:`page-howto-dispersion-run`）。

4. パネル法や CFD の表は作成に時間がかかるので、``aero`` で一度作り、``--aero-table`` で以後の計算に渡す。

   .. code-block:: sh

      ignisyeet aero sample.toml --set aero.method=panel --out runs/aero_panel
      ignisyeet sim  sample.toml --set aero.method=panel --aero-table runs/aero_panel \
          --set wind.speed=7 --set wind.direction_deg=90 --out runs/sim_panel_7ms

   1 行目が係数表 ``runs/aero_panel/aero_table.csv`` を書く（サンプルのパネル法で 9 秒ほど。20 スレッドの CPU、既定のパネル数）。
   2 行目は表を作り直さずに飛翔を計算する。パネル法の表を使ったサンプルの着地距離は、風速 1、4、7 m/s の東風でそれぞれ 2477.4 m、8019.8 m、13480.1 m である。

   ``--aero-table`` に渡す表は **同じ機体と空力の設定で作ったもの** でなければならない。合わない表を渡すとエラーで止まる（:ref:`IY-O004`）。表を作ったときと同じ ``--set`` で ``aero.*`` を指定するか、``--aero-table`` を外して作り直す。

   ``--aero-table`` は ``aero.method = "table"`` や ``aero --force`` とは併用できず、エラーになる（:ref:`IY-O002`）。

.. _sec-output-lock:

同じ出力先を同時に使えないとき
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

1 つの出力先で同時に動かせる実行は 1 つである。別の実行が動いている出力先に ``geom``、``aero``、``sim``、``dispersion`` を実行すると、何も書かずに次のエラーで止まる（終了ステータス 1。パスと番号は例）。

.. code-block:: text

   error[IY-O031]: another ignisyeet run is using runs/x (lock file runs/x/.ignisyeet.lock, pid 12345); wait for it or use a different --out (if no other run is active, delete runs/x/.ignisyeet.lock)

同時に計算するときは ``--out`` で出力先を分ける。前の実行が強制終了された場合など、動いている実行がないのにこのエラーが出るときは、メッセージの末尾にあるとおり ``.ignisyeet.lock`` を手で消す。

.. _page-howto-dispersion-run:

落下分散を実行する
------------------

風の条件で着地点がどれだけ広がるかを ``ignisyeet dispersion`` で計算し、結果を読む。
落下分散には、風だけを変える **風の格子**\ （``mode = "wind_grid"``、既定）と、推力・質量・空力係数・射角・風などのばらつきを同時に与える **モンテカルロ** 解析（``mode = "monte_carlo"``）がある。どちらを選ぶかは :ref:`page-theory-dispersion` にある。
実行の前に、``sim`` が終了ステータス 0 で終わり、風と回収系の設定が済んでいるようにする。高度別の表を使うなら、頂点より上まで続く表を用意する。
例は ``examples/sample.toml``\ （射角 85 deg、射方位 270 deg、北風 4 m/s、風のモデルは ``power``）である。保安距離を求める手順は :ref:`page-howto-keepout-zone` にある。

風の格子を実行する
~~~~~~~~~~~~~~~~~~

``[dispersion]`` に基準風速の一覧 ``wind_speeds`` と方位の数 ``directions`` を書く。

.. code-block:: toml

   [dispersion]
   mode = "wind_grid"
   wind_speeds = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]   # [wind] の基準高さでの風速 [m/s]
   directions = 8                                      # 北から等間隔の方位の数

``wind_speeds`` の意味は風モデルで異なる。

* ``constant``、``power``、``log``: ``wind.ref_height`` での風速。頂点が 1000 m を超えると、外挿の警告が出る。
* ``profile``: 表の **平均風速** :math:`W`\ （地上の風速ではない）。表の形を保ったまま全体を :math:`v/W` 倍に拡大する。地上風の測定値をそのまま入れてはならない。平均風速がほぼ 0 の表はエラーになる。

ケース数は、風速の数、方位の数、降下モードの数（弾道落下とパラシュート降下の 2 つ）の積である。上の設定は :math:`7\times8\times2=112` ケースである。

.. code-block:: console

   $ ignisyeet dispersion examples/sample.toml

   Dispersion: 112 cases in 997.30ms -> out/dispersion.csv, out/dispersion_meta.json
     Ballistic: max landing distance 1549.6 m
     Parachute: max landing distance 14315.6 m

``[output] kml = true`` なら ``out/ignisyeet.kml`` も書かれる。時間は CPU により異なる（コストの目安は :ref:`page-theory-dispersion`）。


.. _sec-wind-grid-result:

風の格子の結果を読む
~~~~~~~~~~~~~~~~~~~~

:numref:`fig-dispersion` は、同じ風速で風向だけを変えた着地点を線で結んだもので、内側から外側へ風速が大きくなる。
着地点が風上側と風下側のどちらにずれるか、風速とともにどれだけ広がるかは :ref:`page-theory-descent` にある。

.. _fig-dispersion:

.. figure:: /_generated/plots/dispersion.*
   :width: 100%

   サンプル機体の落下分散（射角 85°、射方位 270°、7 風速 × 8 方位）。左が弾道落下、右がパラシュート降下。多角形の中心の位置と、風速に応じた広がりの向きを見る。

* **最大着地距離は、計算した条件の中の最大値である。** 風速と方位の刻みを変えると値が変わりうるので、刻みを細かくして確かめる（:ref:`sec-dispersion-grid-fineness`）。
* **多角形は近似である。** 方位が粗いと着地点が多角形（と KML の輪郭）の外に出る。保安域の判断には、最大着地距離を半径とする円か、方位を 36 以上にした外側の包絡を使う。
* **弾道落下にはいびつな点が出ることがある。** 計算の誤りではない。安全の評価では、このような点を含めた最も外側の範囲で考える。

モンテカルロ解析を実行する
~~~~~~~~~~~~~~~~~~~~~~~~~~

``mode`` を ``"monte_carlo"`` にし、``[dispersion.monte_carlo]`` にサンプル数 ``samples`` と乱数の種 ``seed`` を書く。各ばらつきの標準偏差は同じ表に書き、0 にするとそのばらつきは無効になる。
ばらつきの項目、単位、既定の標準偏差は :numref:`tbl-mc-perturb` にある。標準偏差は利用者が決める値で、既定値は目安にすぎない。同じ ``seed`` なら毎回同じ結果になる。
保安距離の検討ではサンプル数を :math:`10^4` 以上にする。

.. code-block:: toml

   [sim]
   integrator = "rk45"

   [dispersion]
   mode = "monte_carlo"

   [dispersion.monte_carlo]
   samples = 1000
   seed = 1
   # truncate_sigma = 3.0   # 指定すると、標準偏差の 3 倍を超えるばらつきを引かない

.. code-block:: console

   $ ignisyeet dispersion examples/sample.toml --set sim.integrator=rk45 --set dispersion.mode=monte_carlo

   Monte Carlo: 1000 samples x 2 descent modes in 1.12s -> out/dispersion_mc.csv, out/dispersion_summary.json
     Ballistic: 1000 ok, 0 failed or did not land
       mean landing E -1080.3 m, N 369.6 m; apogee 4480.0 +- 487.4 m
       1-sigma ellipse 237.0 x 149.3 m, 3-sigma 710.9 x 448.0 m, major axis bearing 131 deg
       max landing distance 2141.5 m (sample 935)
       landing distance P50 1131.1 m, P90 1458.9 m, P99 1848.3 m, P99.9 1999.5 m (landed samples)
     Parachute: 1000 ok, 0 failed or did not land
       mean landing E -651.7 m, N -7663.4 m; apogee 4480.0 +- 487.4 m
       1-sigma ellipse 2185.1 x 2066.8 m, 3-sigma 6555.4 x 6200.3 m, major axis bearing 16 deg
       max landing distance 18018.1 m (sample 830)
       landing distance P50 7835.0 m, P90 10907.1 m, P99 13666.4 m, P99.9 14999.4 m (landed samples)

結果を :numref:`fig-dispersion-mc` と :numref:`tbl-mc-result` に示す（``examples/out/mc``、1000 サンプル、20 スレッドのマシンで 1.1 s）。

.. _fig-dispersion-mc:

.. figure:: /_generated/plots/dispersion_mc.*
   :width: 100%

   モンテカルロ解析による着地点の分布。点が各サンプルの着地点、十字が平均、実線と破線が :math:`1\sigma`、:math:`3\sigma` の誤差楕円、星が射点。左が弾道落下、右がパラシュート降下（どちらも 1000 点）。点の広がりと楕円の向きを見る。

.. _tbl-mc-result:

.. list-table:: モンテカルロ解析の結果（サンプル、1000 サンプル、``seed = 1``）
   :header-rows: 1
   :widths: 40 30 30

   * - 量
     - 弾道落下
     - パラシュート降下
   * - 有効サンプル数 / 失敗
     - 1000 / 0
     - 1000 / 0
   * - 着地点の平均 東 / 北 [m]
     - −1080.3 / 369.6
     - −651.7 / −7663.4
   * - :math:`1\sigma` 楕円 長半径 × 短半径 [m]
     - 237.0 × 149.3
     - 2185.1 × 2066.8
   * - :math:`3\sigma` 楕円 長半径 × 短半径 [m]
     - 710.9 × 448.0
     - 6555.4 × 6200.3
   * - 長軸の方位 [deg]
     - 131
     - 16
   * - 着地距離 P50 / P90 / P99 / P99.9 [m]
     - 1131.1 / 1458.9 / 1848.3 / 1999.5
     - 7835.0 / 10907.1 / 13666.4 / 14999.4
   * - 最大着地距離 [m]（サンプル番号）
     - 2141.5（935）
     - 18018.1（830）
   * - 頂点高度の平均 ± 標準偏差 [m]
     - 4480.0 ± 487.4
     - 4480.0 ± 487.4

* **平均と楕円。** 弾道落下の平均着地点は射点から西に 1080 m、北に 370 m にあり、パラシュート降下の広がり（:math:`1\sigma`）は約 2 km で弾道落下より 1 桁大きい。楕円の定義は :ref:`fig-error-ellipse` にある。
* **楕円の内側の割合。** 正規分布なら :math:`1\sigma` 楕円に約 39 %、:math:`3\sigma` 楕円に約 99 % が入る。この計算では弾道落下で 44 % と 98 %、パラシュート降下で 41 % と 99 % で、分布は正規分布から少しずれている。
* **裾はパーセンタイルと最大値で見る。** 保安域の半径は、楕円ではなく P90、P99、P99.9 と最大着地距離で考える（:ref:`sec-mc-percentile`）。P99.9 は着地したサンプルが 1000 以上のときだけ出る。最大値はサンプル数とともに大きくなる（:ref:`tbl-safety-samples`）。
* **頂点高度の平均。** 4480 m は公称の飛翔（4440 m）より 40 m ほど高い。頂点高度は推力や抗力係数に対して線形ではないので、平均は公称値と一致しない。

出力ファイルの列と項目は :ref:`page-reference-output-files` にある。

Google Earth で見る
~~~~~~~~~~~~~~~~~~~

``[output] kml`` が ``true``\ （既定）のとき ``ignisyeet.kml`` が書かれる。Google Earth で開くと、射点を中心に地形の上へ表示される。同じ出力先に ``sim`` の結果があれば、飛翔経路もフォルダ ``Flight`` に入る。

フォルダ ``Dispersion – wind grid``\ （``mode = "wind_grid"``）
   降下モードごとのフォルダの下に風速ごとのフォルダがあり、チェックボックスで表示を切り替えられる。風速ごとに、着地点を風向の順に結んだ輪郭と、各ケースの着地点の印が入る。印をクリックすると、風速・風向、着地点の東・北方向の距離、着地時刻と着地速度、頂点高度が見られる。

フォルダ ``Dispersion – Monte Carlo``\ （``mode = "monte_carlo"``）
   降下モードごとに、平均着地点、:math:`1\sigma` と :math:`3\sigma` の誤差楕円、各サンプルの着地点が入る。サンプル数が多いときは、フォルダ ``Landing points`` を切ると楕円だけを見られる。失敗したサンプルと着地しなかったサンプルは含まれない。

風の格子の輪郭は着地点を直線で結んだものなので、保安域の確認には着地点の印も見る。
``sim`` と ``dispersion`` で ``[sim]`` を含む設定が違うと、飛翔経路と落下分散は 1 つの KML にそろわない（:ref:`sec-kml`）。同じ出力先で重ねるときは、設定をそろえて両方を実行し直す。
図形は地面に固定して描かれ、着地点は地形の起伏を考えない射点標高の面で決まる（:ref:`page-theory-limitations`）。

.. _sec-wind-grid-failed:
.. _sec-mc-failure:
.. _sec-dispersion-unstable:
.. _sec-dispersion-broadside:

結果が使えるかを確かめる
~~~~~~~~~~~~~~~~~~~~~~~~

終了ステータスが 0 なら、すべてのケースまたはサンプルが完了し、安定で、係数表の範囲内である。警告は ``-q`` でも表示され、風の格子では ``dispersion_meta.json``、モンテカルロでは ``dispersion_summary.json`` の ``warnings`` に残る。
3 のときは出力が書かれていても結果は完全でないか使えず、標準エラー出力の ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行に理由とコードが出る（:ref:`sec-exit-status`）。

症状ごとの結果の扱いは次のとおりである。終了ステータス 3 の結果は保安の判断に使わない。

.. list-table::
   :header-rows: 1
   :widths: 24 38 38

   * - 症状
     - 結果の扱い
     - コードと対処
   * - 着地しないケースやサンプルがある（``sim.max_time`` までに着地しない、推力が小さすぎてランチャを離れない、レールの射角が範囲外など）
     - そのケースは最大着地距離、統計、KML の輪郭から除かれる。最大着地距離は下限（``≥``）で、``complete`` が ``false`` になる。全部が着地しなければ距離は出ない
     - :ref:`IY-O028`、:ref:`IY-O530`。``sim.max_time`` を延ばすか、失敗の理由を直す。``dispersion.csv`` の ``error`` 列と ``dispersion_mc.csv`` の ``status`` 列を読む
   * - 積分がエラーで止まったケースがある
     - 失敗として数える。すべてが止まると終了ステータス 1 になる
     - :ref:`IY-O531`、:ref:`IY-O011`
   * - 公称の飛翔が不安定か、安定性を判定できない
     - 結果は使えない
     - :ref:`IY-O020`、:ref:`IY-O021`。機体の設定を直す
   * - 飛翔が係数表の Mach 数または迎角の範囲を外れた
     - 結果は使えない
     - :ref:`IY-O022`、:ref:`IY-O024`。表の範囲を広げて作り直す
   * - 着地したサンプルが少ない
     - 1 つだけなら平均着地点だけ、2 未満なら楕円は出ない。30 未満の統計は粗い
     - :ref:`IY-O538`。サンプル数を増やす
   * - 静的に不安定なケースがある
     - 統計と最大着地距離に入れたままにする。不安定な機体は転倒して飛び、除くと裾を過小に見積もるためである。``stable`` が ``false`` の行が該当する条件である
     - :ref:`IY-O025`、:ref:`IY-O534`。静安定余裕を増やすか、重心や空力係数の標準偏差を見直す
   * - 横向きに落ちた弾道落下がある（迎角 60° から 120° で 10 s を超えて飛ぶ。``broadside_runs`` と ``broadside_time``）
     - 統計に入れたままにし、終了ステータスは変わらない。この着地点は検証していない大迎角の空力係数で決まる（:ref:`page-theory-limitations`）
     - :ref:`IY-O536`。距離を除かず、空力の手法を替えて計算し（:ref:`page-howto-keepout-zone` の手順 6）、大きいほうを使う

次の警告も出る。``warning[IY-O512]`` は頂点が風の表の最上端より上であることを示し、最も高い頂点より上まで続く表で計算し直す。
``warning[IY-O514]`` は ``power`` または ``log`` の風が外挿値であることを示し、保安の検討には高度別の表を使う。
``warning[IY-O518]`` は開傘の速さが 50 m/s または 2 kPa を超えることを示す。着地点はパラシュートが壊れないものとして計算しているので、この開傘は弾道落下に近い降下になりうるものとして扱う。

.. _page-howto-keepout-zone:
.. _sec-dispersion-safety:

保安距離を求める
----------------

落下分散の結果を保安距離（保安域の半径）の根拠にするときの手順を示す。
1 回の ``ignisyeet dispersion`` の最大着地距離は、計算した風速・風向・サンプルの中での最大値にすぎない。計算していない条件の着地点は、これより遠くなりうる。
そこで、遠い着地点を取りこぼさないように条件の組合せを広げ、すべての最大値の最大に余裕を足して半径とする。具体的には、bash のコマンドで次を順に行う。

* 打上げ可能な風速の上限までの風の格子を、最大値が変わらなくなるまで細かくする。
* 格子で最も遠くなった風で、機体のばらつきのモンテカルロ解析を行う。予報の風でのモンテカルロ解析も行う。
* 二段開傘の機体では、正常な開傘に加えて、メインが頂点で開く場合とドローグだけで降りる場合を計算する。
* 以上を部品積み上げ法とパネル法の両方で計算する。

手順の全体を :numref:`fig-keepout-flow` に示す。
保安距離そのものの規定は、打上げを行う場所と団体の規則に従う。ここでは規定の数値は与えない。
この手順の根拠（サンプル数と最大値の関係、条件を重ねる範囲）は :ref:`page-theory-dispersion` にある。結果を使う前に :ref:`page-theory-limitations` も読む。

.. _fig-keepout-flow:

.. figure:: /_generated/tikz/keepout_flow.*
   :width: 100%

   保安距離を求める手順 0 から 7 の流れ。上段の青い箱は公称の機体の計算と風の探索、下段の青い箱は機体のばらつきと空力の手法の違いを含める計算、橙の箱は二段開傘の機体だけの計算である。すべての計算の最大値の最大に余裕を足して半径とする。

``sim`` が正常に終わり、風と回収系の設定が済んでいて、打上げを中止する風速（ランチ風速の上限）とその風速を測る高さが決まっている状態から始める。
保安の検討には高度別の表（``wind.model = "profile"``）を使う。``power`` と ``log`` は地上の基準風速を上空へ外挿するので、上空の風が強い日の着地点を過小にも過大にも見積もる。例の ``examples/wind_profile_example.csv`` は書式を示す架空の表なので、実際には当日の予報や観測の表に置き換える。
**bash** と ``python3`` を使う。zsh などを使っているときは先に ``bash`` を起動する。

計算時間の目安：例の高度別の表で手順 0 から 4 のすべてを ``rk45`` で計算すると、20 スレッドの CPU で約 8 分かかる。``rk4`` のままだと 1 時間半ほどかかるので、下の ``P`` に ``--set sim.integrator=rk45`` を加える。4 コアのノート PC では数十分から 1 時間以上を見込む。測定の条件は :ref:`page-vv-performance` にある。

1 つの設定ファイルを書き換えずに、``--set``、``--out``、``--aero-table`` （:ref:`page-howto-sweeps`）で条件を変えて実行する。
``--set`` の組は bash の配列に入れ、``"${P[@]}"`` の形で展開する。こうすると、``[7.8, 11.9]`` のように空白や角かっこを含む値も 1 つの引数のまま渡る。結果は現在のディレクトリの ``runs/`` の下に計算ごとに分けて書く。

``ignisyeet`` を ``PATH`` に入れ、設定ファイルのパスを ``C`` に入れる。ソースからビルドしたときは、リポジトリの最上位のディレクトリで次を実行する。インストーラで入れたときは ``~/.local/bin`` が ``PATH`` に入っていればよく、例題をコピーしたディレクトリの設定ファイルを ``C`` にする。以下のコマンドは、``runs/`` を作りたいディレクトリで実行する。

.. code-block:: bash

   export PATH="$PWD/target/release:$PATH"   # ソースからビルドしたとき
   C=examples/sample.toml                    # インストーラなら
                                             # C=ignisyeet-examples/sample.toml

関数 ``run`` は ``ignisyeet`` を実行し、終了ステータスが 0 でなかったコマンドを ``runs/status.txt`` に記録する（手順 7 で確かめる）。

.. code-block:: bash

   mkdir -p runs && : > runs/status.txt
   run() {
     ignisyeet "$@" || echo "exit $?: ignisyeet $*" >> runs/status.txt
   }

関数 ``worst`` は、出力先と降下モード（``ballistic`` または ``parachute``）を与えると、格子で最も遠くなったケースの風速と風向を出す。

.. code-block:: bash

   worst() {
     python3 - "$1/dispersion.csv" "$2" <<'EOF'
   import csv, sys
   f, d = sys.argv[1:]
   rows = [r for r in csv.DictReader(open(f))
           if r["landed"] == "true" and r["descent"] == d]
   r = max(rows, key=lambda r: float(r["landing_distance"]))
   print(r["wind_speed"], r["wind_direction_deg"])
   EOF
   }

高度別の風の表は ``wind.profile`` で与え、風を変えるときは ``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` で表全体を拡大・回転する（:ref:`sec-wind-scaling`）。
サンプルの設定ファイルは ``speed`` と ``direction_deg`` を書いているので、以下のコマンドでは ``warning[IY-C502]`` が毎回出る。結果には影響しない。自分の設定ファイルでは ``[wind]`` からこの 2 つを消しておく。

すべての計算に共通の ``--set`` を配列 ``P`` に、二段開傘の正常な開傘の 2 項目を配列 ``N`` に入れる。
``--set`` は項目を消せないので、二段開傘の機体では ``recovery.main_cd_s`` と ``recovery.main_altitude`` を設定ファイルに書かず（ファイルの ``recovery.cd_s`` はドローグの抗力面積）、手順 0 から 4 のすべての計算に ``"${N[@]}"`` で加える。
こうすると、手順 5 では ``N`` を除く（ドローグだけ）か、``N`` の代わりにメインが頂点で開く設定を入れて、故障を飛ばせる。単一のパラシュートの機体では ``N=()`` とする。

.. code-block:: bash

   P=(--set wind.model=profile
      --set wind.profile=wind_profile_example.csv)
   N=()   # 単一のパラシュート

二段開傘の機体として試すとき（ドローグ 0.5 m\ :sup:`2`、高度 300 m でメイン 8 m\ :sup:`2`）は、続けて次を実行する。

.. code-block:: bash

   P+=(--set recovery.cd_s=0.5)   # ファイルの cd_s をドローグの値に
   N=(--set recovery.main_cd_s=8 --set recovery.main_altitude=300)

KML が要らない一括の計算では、``P`` に ``--set output.kml=false`` を加えると出力が小さくなる（結果は変わらない）。

手順
~~~~

各手順の計算と、使う回収系の設定を :numref:`tbl-safety-steps` にまとめる。

.. _tbl-safety-steps:

.. list-table:: 手順と回収系の設定
   :header-rows: 1
   :widths: 10 46 44

   * - 手順
     - 計算
     - 回収系
   * - 0
     - 係数表と公称の飛翔
     - 正常な開傘（``N``）
   * - 1、2
     - 風の格子（36 方位、細分した格子）
     - 正常な開傘（``N``）
   * - 3
     - 手順 2 の格子で最も遠くなった風での、機体のばらつきのモンテカルロ解析
     - 正常な開傘（``N``）
   * - 4
     - 予報の風でのモンテカルロ解析
     - 正常な開傘（``N``）
   * - 5
     - 故障ごとの風の格子と、その格子で最も遠くなった風でのモンテカルロ解析
     - メインが頂点で開く（``A``）、ドローグだけ（``N`` なし）。二段開傘の機体だけ
   * - 6
     - 空力の手法の違い（手順 0 から 5 を両方の手法で）
     - 各手順のとおり
   * - 7
     - 終了ステータスと警告の確認、半径の決定
     - ―

**手順 0　係数表を作り、表の平均風を確かめる。**
空力の手法ごとに係数表を一度だけ作り、以後の計算は ``--aero-table`` でそれを使う（手順 6 のため、部品積み上げ法とパネル法の両方を作る）。

.. code-block:: bash

   for m in barrowman panel; do
     run aero "$C" --set aero.method=$m --out runs/aero_$m
   done
   run sim "$C" "${P[@]}" "${N[@]}" \
       --aero-table runs/aero_barrowman --out runs/nominal

``runs/nominal/summary.json`` に表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` が ``wind_speed`` と ``wind_direction_deg`` として出る（:ref:`sec-wind-profile`）。
``wind.ref_height`` での表の風（手順 1 で使う）は ``wind_at_ref_height`` に出て、端末には ``profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)`` の行が出る。例の表では :math:`W=11.89` m/s、:math:`\Psi=220.2^\circ` である。
開傘時の対気速度と動圧（手順 5）もこの ``summary.json`` に出る。

**手順 1　風の格子を、打上げ可能な風速の上限まで、36 方位以上で計算する。**
``dispersion.wind_speeds`` は打上げを中止する風速までを含め、``directions`` は 36 以上（10° 間隔以下）にする。
``wind_speeds`` の意味は風モデルで異なる。

* ``constant``、``power``、``log``: ``wind.ref_height`` での風速である。団体が風速の上限を測る高さに合わせて ``ref_height`` を設定し、その高さでの上限までを含める。
* ``profile``: 表の **平均風速** :math:`W` である。団体の上限 :math:`V_{\mathrm{lim}}`\ （測る高さ :math:`h_{\mathrm{lim}}` での風速）を表に当てはめるには、表のその高さの風速 :math:`w(h_{\mathrm{lim}})` を求め、上限に当たる格子の風速を :math:`v_{\max}=W\,V_{\mathrm{lim}}/w(h_{\mathrm{lim}})` とする。
  :math:`h_{\mathrm{lim}}` を ``wind.ref_height`` に合わせておけば、:math:`w(h_{\mathrm{lim}})` は手順 0 の ``wind_at_ref_height`` の ``speed`` である。違う高さで測るときは、表のその高さの風速を前後の行の線形補間で読む。
  格子には :math:`v=W`\ （表そのまま、:math:`k=1`）と :math:`v_{\max}` までの値を入れる。
  :math:`k=v/W` が 3 を超える、または 1/3 を下回る風速があると警告が出る。拡大が現実的でないので、上限の風速を与えた着地点の上空の風速を確かめ（``wind_scale`` と警告に出る上空の風速）、ふさわしくなければ、上限の風速ごとに別の表を用意する（:ref:`sec-wind-scaling`）。

例として、上限を地上 2 m（既定の ``wind.ref_height``）で 7 m/s とする。手順 0 の出力から :math:`w(2\ \mathrm{m})=3.04` m/s、:math:`v_{\max}=11.89\times7/3.04=27.4` m/s である。
格子には、地上 2 m で 2、4、5、6、7 m/s に当たる 7.8、15.6、19.6、23.5、27.4 m/s と、:math:`W` そのものの 11.9 m/s を入れる（:math:`k=0.66` から 2.3）。
格子の風速は配列 ``G`` に入れ（手順 2 と 5 でも使う）、最大の風速を ``VMAX`` に入れる（手順 3 で使う）。

.. code-block:: bash

   G=(--set 'dispersion.wind_speeds=[7.8, 11.9, 15.6, 19.6, 23.5, 27.4]')
   VMAX=27.4   # 格子の最大の風速
   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.directions=36 --out runs/grid_$m
   done

各出力先の ``dispersion.csv`` と ``dispersion_meta.json`` が書かれ、降下モードごとの最大着地距離が表示される。

**手順 2　格子を細かくして、最大値が変わらないことを確かめる。**
方位を増やした格子（72、360 など）と比べ、さらに最大となったケースの風速と風向の近くを細かくした格子（風速の刻みを小さくし、方位を 360 まで増やす）でも計算する。最大値が変わる間は細かくし、最も大きい値を使う。
最大は風速の上限で起きるとは限らない。弾道落下では途中の風速で最も遠くなることがある。多角形と KML の輪郭は近似なので、保安域は最大着地距離の円か、細かい格子の外側の包絡で考える（:ref:`sec-wind-grid-result`）。

.. code-block:: bash

   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.directions=360 --out runs/grid360_$m
   done
   worst runs/grid360_panel ballistic   # 例：27.4 116

最も遠いケースの風速と風向は ``worst`` で降下モードごとに読む（``profile`` では平均風速と平均風の風向）。
風速の刻みは、最も遠いケースの風速の前後を細かくした ``wind_speeds`` で計算して細かくする（例は 23.5 から 27.4 m/s を約 0.5 m/s 刻みにしたパネル法の格子）。

.. code-block:: bash

   run dispersion "$C" "${P[@]}" "${N[@]}" --set aero.method=panel --aero-table runs/aero_panel --set dispersion.directions=360 --set 'dispersion.wind_speeds=[23.5, 24, 24.5, 25, 25.5, 26, 26.5, 27, 27.4]' --out runs/grid360fine_panel

刻みを細かくしても最大着地距離が変わらない。増えたときは細かい格子の値を使い、手順 3 ではその出力先（例では ``runs/grid360fine_panel``）を ``worst`` に与える。

**手順 3　機体のばらつきを、最も遠くなる風で飛ばす。**
機体のばらつきと強い風が重なった着地点を求める。降下モードと空力の手法ごとに、手順 2 の格子で最も遠くなったケースの風を公称の風にして、``mode = "monte_carlo"`` で計算する。故障の場合の最も遠い風は故障ごとに違うので、手順 5 で別に求める。

* ``profile`` では、そのケースの風速と風向を ``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` に与える。公称の飛翔は格子のそのケースと同じ風になる。``constant``、``power``、``log`` では ``wind.speed`` と ``wind.direction_deg`` に与える。
* 風は格子で調べたので、``wind_speed`` と ``wind_direction_deg`` の標準偏差は 0 にし、推力、質量、空力係数、レールの向きなどの機体のばらつきだけを与える。
* 機体のばらつきで最も遠くなる風向がずれることがあるので、上限の風速でほかの風向（たとえば 30° ごと）も計算する。風の格子は公称の機体だけを飛ばすので、機体のばらつきと強い風が重なった着地点はこの手順でしか出ない。
* サンプル数は :math:`10^4` 以上にする。最大値はサンプル数とともに大きくなるので、P99 と P99.9 と最大値を併記し、サンプル数を 2 から 4 倍にした計算と比べる（:ref:`tbl-safety-samples`）。

ばらつきの配列 ``MC`` は手順 5 でも使う。

.. code-block:: bash

   MC=(--set dispersion.mode=monte_carlo
       --set dispersion.monte_carlo.samples=10000
       --set dispersion.monte_carlo.wind_speed=0
       --set dispersion.monte_carlo.wind_direction_deg=0)
   for m in barrowman panel; do
     for k in ballistic parachute; do
       read -r v d < <(worst runs/grid360_$m $k)
       run dispersion "$C" "${P[@]}" "${N[@]}" "${MC[@]}" \
           --set aero.method=$m --aero-table runs/aero_$m \
           --set wind.profile_mean_speed="$v" \
           --set wind.profile_mean_direction_deg="$d" \
           --out runs/mc_worst_${m}_$k
     done
   done
   # 上限の風速で 30° ごとの風向
   for m in barrowman panel; do
     for d in $(seq 0 30 330); do
       run dispersion "$C" "${P[@]}" "${N[@]}" "${MC[@]}" \
           --set aero.method=$m --aero-table runs/aero_$m \
           --set wind.profile_mean_speed="$VMAX" \
           --set wind.profile_mean_direction_deg="$d" \
           --out runs/mc_${m}_$d
     done
   done

各 ``dispersion_summary.json`` の ``wind_speed``、``wind_direction_deg``、``wind_scale`` が、意図した公称の風になっている。

**手順 4　予報の風でモンテカルロ解析を行い、標準偏差の根拠を残す。**
当日の予報の表そのものを公称の風にして、風のばらつき（予報の誤差）も含めて計算する。``profile`` の風速の標準偏差は平均風速 :math:`W` の周りの揺らぎで、表全体を拡大する。
標準偏差は利用者が決める値で、既定値は目安にすぎない。推力は燃焼試験のばらつき、空力係数は手法どうしの差、風は予報の誤差のように、根拠を決めて設定する。

.. code-block:: bash

   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.mode=monte_carlo \
         --set dispersion.monte_carlo.samples=10000 \
         --out runs/mc_forecast_$m
   done

``runs/mc_forecast_*/dispersion_summary.json`` に、降下モードごとの最大着地距離と P99、P99.9 が出る。

**手順 5　回収系の故障を飛ばす。**
弾道落下（パラシュートが開かない場合）は手順 1 から 4 で常に計算されるので、その最大値を含める。弾道落下は回収系の設定によらない。単一のパラシュートの機体では、この手順はこれだけである。
二段開傘の機体では、さらに次の 2 つの故障を、手順 2 と同じ格子と、その格子でパラシュート降下が最も遠くなった風でのモンテカルロ解析（手順 3 と同じばらつき）で計算する。

* **メインが頂点で開く**\ （配列 ``A``）: ``main_altitude`` を頂点より高くする（例えば 100000）。メインがドローグと同時に開き、``main_altitude`` が頂点より高いという警告が出る。
* **メインが開かず、ドローグだけで降りる**: ``N`` を加えない。

``A`` のメインの抗力面積は ``N`` と同じ値にする。

.. code-block:: bash

   A=(--set recovery.main_cd_s=8 --set recovery.main_altitude=100000)
   for f in apogee drogue; do
     F=()                                  # drogue：ドローグだけ
     if [ "$f" = apogee ]; then F=("${A[@]}"); fi
     for m in barrowman panel; do
       run dispersion "$C" "${P[@]}" "${F[@]}" "${G[@]}" \
           --set aero.method=$m --aero-table runs/aero_$m \
           --set dispersion.directions=360 \
           --out runs/grid360_${f}_$m
       read -r v d < <(worst runs/grid360_${f}_$m parachute)
       run dispersion "$C" "${P[@]}" "${F[@]}" "${MC[@]}" \
           --set aero.method=$m --aero-table runs/aero_$m \
           --set wind.profile_mean_speed="$v" \
           --set wind.profile_mean_direction_deg="$d" \
           --out runs/mc_worst_${f}_$m
     done
   done

風による流されは降下時間が長いほど大きい。メインが頂点で開くと降下時間が最も長く、着地点は風下へ最も遠い。
ドローグだけの降下は、ドローグの抗力面積が小さく降下が速ければ、正常な二段開傘より **近く** に落ちる（着地速度は大きい）。ドローグが大きく降下が遅ければ遠くなる。思い込みで除かず、計算して大きいほうを取る。
開傘時の対気速度と動圧は ``summary.json`` の ``drogue_deploy_speed`` ``drogue_deploy_q`` ``main_deploy_speed`` ``main_deploy_q`` に出る。50 m/s または 2 kPa を超える開傘には警告が出る（:ref:`IY-O518`）。着地点はパラシュートが壊れないものとして計算しているので、この警告が出る開傘は、弾道落下に近い降下になりうるものとして扱う。

**手順 6　空力の手法の違いを含める。**
空力係数は実測と比較しておらず（:ref:`page-theory-limitations`）、手法の差は最大着地距離に大きく効く。``aero.method = "barrowman"`` と ``"panel"`` の両方で手順 1 から 5 を計算し、大きいほうの最大値を使う。
公称の飛翔の着地点の差（サンプルの弾道落下で約 69 m）は小さくても、風の格子とモンテカルロの最大値の差は大きくなりうる。``cn_scale`` の標準偏差は 1 つの手法の中の不確かさで、手法の差はこの手順で含める。
手順 1 から 5 の出力先が、両方の手法でそろう（``runs/*_barrowman`` と ``runs/*_panel``）。

**手順 7　終了ステータスと警告を確かめ、半径を決める。**
``runs/status.txt`` が空であること（すべてのコマンドの終了ステータスが 0 であること）を確かめる。ここに書かれた計算の結果は保安の判断に使わない（:ref:`sec-exit-status`）。
着地しなかったケース、失敗したケース、静的に不安定なケース、係数表の迎角の範囲外の引き出しがあると終了ステータス 3 になる。そのときの最大着地距離は下限である（:ref:`sec-wind-grid-failed`）。
警告は ``-q`` でも表示され、``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` に残る。開傘の速さ、静安定余裕の不足、``power`` と ``log`` の外挿、表の拡大率、横向きに落ちた弾道落下（:ref:`sec-dispersion-broadside`）も見る。
特に、頂点が風の表の最上端より上にあるという警告が出たら、最も高い頂点より上まで続く表で計算し直す（:ref:`IY-O512`）。

最大着地距離は、風の格子では ``dispersion.csv``、モンテカルロ解析では ``dispersion_summary.json`` の ``max_landing_distance`` にある。
次のスクリプトは、計算ごとに最大着地距離（モンテカルロ解析では P99 と P99.9 も）、横向きに落ちた弾道落下の数 ``broadside_runs``、警告 ``warnings`` をまとめて出す。

.. code-block:: bash

   cat runs/status.txt   # 何も出なければよい
   python3 - <<'EOF'
   import csv, glob, json, os

   def notes(j):
       print("  broadside_runs:", j.get("broadside_runs"))
       for w in j.get("warnings", []):
           print("  warning:", w)

   for f in sorted(glob.glob("runs/*/dispersion.csv")):
       rows = [r for r in csv.DictReader(open(f))
               if r["landed"] == "true"]
       for d in ("ballistic", "parachute"):
           v = [float(r["landing_distance"])
                for r in rows if r["descent"] == d]
           if v:
               print(f, d, "max", max(v))
       meta = os.path.join(os.path.dirname(f),
                           "dispersion_meta.json")
       notes(json.load(open(meta)))
   for f in sorted(glob.glob("runs/*/dispersion_summary.json")):
       j = json.load(open(f))
       for d in j["descents"]:
           print(f, d["descent"],
                 "max", d["max_landing_distance"],
                 "P99", d["landing_distance_p99"],
                 "P99.9", d["landing_distance_p999"],
                 "complete", d["complete"])
       notes(j)
   EOF

``status.txt`` が空で、``complete`` がすべて ``true`` である。半径は、手順 1 から 6 のすべての計算の弾道落下とパラシュート降下の最大着地距離の最大に、余裕を足した値とする。

.. math::

   R = \max\bigl(\text{手順 1〜6 のすべての計算の最大着地距離}\bigr) + \text{余裕}

余裕には、風は定常で突風や乱流を含まないこと、地形を考えず着地を射点標高の面で判定すること、空力係数が未検証であることを見込む（:ref:`page-theory-limitations`）。余裕の大きさは利用者が決める。

半径を決める前に、次の点を確かめる。

* ``runs/status.txt`` が空である。
* すべての ``dispersion_summary.json`` で ``complete`` が ``true`` で、``unstable`` と ``stability_unknown_runs`` が 0 である。
* 風の格子の最大値が、方位と風速の刻みを細かくしても変わらない（手順 2）。
* モンテカルロ解析のサンプル数が :math:`10^4` 以上で、P99、P99.9、最大値を併記している。サンプル数を 2 から 4 倍にしても P99 と P99.9 が変わらない。
  最大値は収まらないので、サンプル数とともに記録する。最大値を決めている飛翔（``max_landing_distance_sample`` の行）のばらつきを ``dispersion_mc.csv`` で見て、その組合せが起こりうるかを判断する。
* 手順 3 と 5 は、上限の風、機体のばらつきの裾、回収系の故障という、それぞれ起こりにくい条件を重ねる。同時に起こる確率は小さく、得られる半径は意図して保守的なものである。どの条件までを重ねるかは、打上げを行う団体が決める。
* ``sim``、風の格子、モンテカルロ解析で ``[sim]`` を含む設定が同じである。違うと KML で飛翔経路・風の格子・モンテカルロの層が 1 つにそろわない。

計算例
~~~~~~

サンプル（射角 85°、射方位 270°）について、手順 1、3、4 を計算した最大着地距離を :numref:`tbl-safety-example` に示す。
再現できるように、高度別の表ではなくサンプル設定の ``power`` の風（風速は地上 2 m、上限 7 m/s）で、``P=(--set sim.integrator=rk45)``、``N=()``、``G=(--set 'dispersion.wind_speeds=[1,2,3,4,5,6,7]')``、``VMAX=7`` として計算した。
手順 3 の風は ``--set wind.speed=7 --set wind.direction_deg=270`` のように ``speed`` と ``direction_deg`` で与える。モンテカルロ解析は :math:`10^4` サンプル、``seed = 1`` である。

.. _tbl-safety-example:

.. list-table:: 保安距離の計算例（サンプル、``power`` の風、最大着地距離 [m]。括弧内はその風速と風向）
   :header-rows: 1
   :widths: 46 27 27
   :class: longtable

   * - 計算
     - 弾道落下
     - パラシュート降下
   * - 手順 1：風の格子（1〜7 m/s、36 方位）、部品積み上げ法
     - 1549.6（7 m/s、270°）
     - 14315.6（7 m/s、90°）
   * - 手順 1：同、パネル法
     - 1616.9（7 m/s、270°）
     - 13480.1（7 m/s、90°）
   * - 手順 3：7 m/s、30° ごとの 12 風向、機体のばらつきだけ、部品積み上げ法
     - 3993.0（270°）
     - 28927.8（90°）
   * - 手順 3：同、パネル法
     - 3935.2（270°）
     - 26219.7（90°）
   * - 手順 4：予報の風（北風 4 m/s、風のばらつきを含む）、部品積み上げ法
     - 2581.8
     - 19116.1
   * - 手順 4：同、パネル法
     - 2547.7
     - 17713.0
   * - 最大
     - 3993.0
     - 28927.8

* 手順 3 の最大（パラシュート降下の 28927.8 m）は、格子の最大（14315.6 m）の約 2 倍である。この着地点は手順 3 でしか出ない（:ref:`page-theory-dispersion`）。
* 手順 2 の細分した格子（360 方位など）でも、格子の最大値は変わらない。
* 半径は :math:`\max(3993.0,\ 28927.8)=28927.8` m に余裕を足した値である。余裕の 10 % は計算例であり、推奨値ではない。

サンプル数と最大値の関係は :numref:`tbl-safety-samples` にある。
エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。
