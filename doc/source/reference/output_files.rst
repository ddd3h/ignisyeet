.. _page-reference-output-files:

出力ファイル
==============

出力先ディレクトリ（``output.dir``）に書かれるファイルの一覧と、形状・飛翔・落下分散の出力の列と項目を示す。
係数表と CFD の出力は :ref:`page-reference-aero-table-format` と :ref:`page-reference-cfd-output-files` にある。
各コマンドは、実行の開始時に、出力先にある同じ種類の前回の結果を削除する。落下分散は同じモードの前回の結果だけを削除し、もう一方のモードの結果は残す。

.. _sec-output-list:

ファイルの一覧
--------------

.. list-table::
   :header-rows: 1
   :widths: 30 20 50
   :class: longtable

   * - ファイル
     - 書くコマンド
     - 内容
   * - ``geometry.json``
     - ``geom`` ``aero``
     - 抽出した形状（:ref:`sec-file-geometry`）
   * - ``profile.csv``
     - ``geom`` ``aero``
     - 断面ごとの胴体半径とフィン高さ（:ref:`sec-file-geometry`）
   * - ``aero_table.csv`` ``aero_table.json``
     - ``aero``
     - 空力係数表と付随ファイル（:ref:`sec-aero-table-format`）
   * - ``aero_drag.csv``
     - ``aero``
     - ``method = "barrowman"`` と ``"panel"``。Mach 数ごとの抗力の内訳（:ref:`sec-file-drag`）
   * - ``panel_cp.csv``
     - ``aero``
     - ``method = "panel"``。表面パネルの圧力係数（:ref:`sec-file-panel-cp`）
   * - ``cfd_cases.csv`` ``cfd_report.json``
     - ``aero``
     - ``method = "cfd"``。ケースごとの結果と実行の要約（:ref:`sec-file-cfd`）
   * - ``<workdir>/``
     - ``aero``
     - ``method = "cfd"``。CFD の作業ディレクトリ（:ref:`sec-file-mesh`）
   * - ``trajectory.csv``
     - ``sim``
     - 飛翔の時刻歴（:ref:`sec-file-trajectory`）
   * - ``summary.json``
     - ``sim``
     - 1 回の飛翔の要約（:ref:`sec-file-summary`）
   * - ``dispersion.csv`` ``dispersion_meta.json``
     - ``dispersion``
     - ``mode = "wind_grid"``。風の各ケースの結果と付随情報（:ref:`sec-file-dispersion`）
   * - ``dispersion_mc.csv`` ``dispersion_summary.json``
     - ``dispersion``
     - ``mode = "monte_carlo"``。サンプルごとの結果と統計（:ref:`sec-file-dispersion`）
   * - ``ignisyeet.kml``
     - ``sim`` ``dispersion``
     - ``output.kml = true`` のとき。Google Earth 用の KML ファイル 1 つ（:ref:`sec-kml`）
   * - ``plots/*.png``
     - ``python/plot.py``
     - 図（:ref:`page-howto-plots-kml`）

形状の出力
----------

.. _sec-file-geometry:

``geometry.json``
~~~~~~~~~~~~~~~~~

``geom`` と、係数表を作る ``aero`` が書く、抽出した形状である。
全長 ``length``、基準半径 ``ref_radius`` と基準面積 ``ref_area``、底面半径 ``base_radius``、ノーズ ``nose``、ボートテール ``boattail``、胴体のぬれ面積 ``wetted_area_body`` と体積 ``volume``、
フィンの平面形の面積 ``planform_area`` と重心位置 ``planform_centroid``、フィン ``fins``、STL の座標との対応 ``frame``\ （``axis_point``、``aft_direction``、``nose_offset``）、断面ごとの ``profile`` を持つ。
機体軸 :math:`x_b,y_b,z_b` を STL の座標で表した単位ベクトル ``body_x``、``body_y``、``body_z`` も書く（:ref:`sec-body-basis`）。
抽出の妥当性についての所見は、文字列の配列 ``warnings`` に書く（疑わしい点がなければ空。:ref:`sec-geometry-warnings`）。各文字列は ``[IY-G5nn] …`` の形で、コードを角括弧のまま先頭に付ける（:ref:`errors`）。

``profile.csv``
~~~~~~~~~~~~~~~

見出しは ``x,r,fin_span`` で、断面の位置 ``x``、胴体半径 ``r``、フィン高さ ``fin_span`` [m] を断面ごとに書く。

飛翔と落下分散の出力
--------------------

飛翔と落下分散の出力の列と項目を示す。

.. _sec-file-summary:

``summary.json``
~~~~~~~~~~~~~~~~

``sim`` が書く、1 回の飛翔の要約である。項目は次のとおりである。長さは m、速さは m/s、時刻は s、角度は deg である。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
   * - ``descent``
     - 降下モード
   * - ``wind_speed`` / ``wind_direction_deg``
     - 基準風の風速 [m/s] と風向 [deg]。``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`\ （:ref:`sec-wind-profile`）
   * - ``rail_exit_time`` / ``rail_exit_speed``
     - ランチャ離脱の時刻 [s] と速度 [m/s]（:ref:`sec-rail`）
   * - ``burnout_time``
     - 燃焼終了時刻 [s]（推力曲線の終わりの時刻）。飛翔がそれより前に終わったときは ``null``
   * - ``max_speed`` / ``max_mach`` / ``max_dyn_pressure``
     - 最大速度 [m/s]、最大 Mach 数、最大動圧 [Pa]
   * - ``apogee_time`` / ``apogee`` / ``apogee_east`` / ``apogee_north``
     - 頂点の時刻 [s]、射点からの高さ [m]、位置（東・北、射点から [m]）
   * - ``min_stability_cal``
     - 自由飛行の最小静安定余裕 [cal]。ランチャ離脱の時刻から頂点までのうち、対気速度が 10 m/s を超える間の最小値で、離脱時の対気速度が 10 m/s を超えるときは離脱時の値を含む。頂点付近の遅い部分と降下は含まない。該当する時刻がなければ ``null``。迎角 0 の極限の小迎角の余裕で、その状態が飛んでいる迎角は入らない（:ref:`sec-static-margin-general`）
   * - ``stability_at_rail_exit``
     - ランチャ離脱時の静安定余裕 [cal]。対気速度によらず求める。軸対称表では :math:`(x_{cp}(\delta)-x_{cg})/d`\ （:eq:`eq-margin-axisym`）、一般表ではピッチとヨーの余裕の小さいほう
   * - ``landing_time``
     - 着地の時刻 [s]（:ref:`sec-landing`）
   * - ``landing_east`` / ``landing_north`` / ``landing_lat`` / ``landing_lon``
     - 着地位置。東・北は射点から [m]、緯度・経度は [deg]
   * - ``landing_distance``
     - 射点からの距離 [m]
   * - ``landing_speed``
     - 着地の瞬間の地面に対する速さ [m/s]。水平方向の流され（風による移動）の速さを含む。降下率（鉛直の速さ）ではなく、パラシュート降下で風が強いと降下率より大きい
   * - ``landed``
     - ``sim.max_time`` までに着地したか（真偽値）。``false`` のとき、着地に関する 7 項目（``landing_time`` から ``landing_speed`` まで）は ``null`` である
   * - ``main_deploy_time`` / ``main_deploy_altitude``
     - 二段開傘（``recovery.main_cd_s`` と ``recovery.main_altitude``）のメインが開傘した時刻 [s] と射点からの高さ [m]（:ref:`sec-dual-deploy`）。
       メインがない設定、弾道落下、メインの開傘より前に飛翔が終わった場合は ``null`` である
   * - ``static_margin_min_cal``
     - ``min_stability_cal`` と ``stability_at_rail_exit`` のうち、求まったものの小さいほう [cal]（結果パネルの ``min margin`` と同じ）。
       ``min_stability_cal`` が ``null``\ （対気速度が 10 m/s を超えなかった）なら ``stability_at_rail_exit`` の値で、どちらも求まらなければ ``null``
   * - ``static_margin_min_t`` / ``static_margin_min_mach``
     - 最小の静安定余裕をとった状態の時刻 [s] と Mach 数（:ref:`sec-static-margin-general`）。余裕を一度も求めなかった飛翔では ``null``
   * - ``stable``
     - 静的に安定なら ``true``、不安定なら ``false``、判定できないなら ``null``（定義は :ref:`sec-exit-definitions`）。
       ``false`` と ``null`` のとき、``sim`` は着地したかどうかにかかわらず終了ステータス 3 で終わる（:ref:`sec-exit-status`）
   * - ``stability_unknown``
     - ``stable`` が ``null`` のときだけ書く。判定できない理由（``at t = 0.50 s: the yaw normal-force slope dCy/dbeta is zero at Mach 0.10`` の形で、最初に余裕が求まらなかった時刻と理由。:ref:`sec-static-margin-general`）
   * - ``static_margin_pitch_min_cal`` / ``static_margin_yaw_min_cal``
     - ピッチ面とヨー面の静安定余裕の、自由飛行中とランチャ離脱時の最小値 [cal]（:ref:`sec-static-margin-general`）。軸対称表では両方とも ``static_margin_min_cal`` と同じ値である。
       その面の余裕がどこかで求まらなければ ``null``
   * - ``drogue_deploy_time``
     - （ドローグ、または単一の）パラシュートが開いた時刻 [s]。開傘は、頂点の時刻に ``recovery.delay`` を足した時刻に、刻み幅によらず正確に起こる（:ref:`sec-integrators`）。その時刻にモータがまだ燃えていれば、推力の終わりの時刻に遅らせる（``deploy_delayed``）。弾道落下と、開傘する前に着地した飛翔では、この項目を書かない
   * - ``deploy_delayed``
     - 開傘を推力の終わりまで遅らせたときだけ書く。本来の開傘時刻（頂点 + ``recovery.delay``）``nominal_time`` [s] と、その時刻の推力 ``thrust`` [N] を持つ（:ref:`sec-parachute`）
   * - ``broadside_time`` / ``fell_broadside``
     - 頂点のあとの弾道落下で、全迎角が 60° から 120° の間で飛んだ時間 [s]（パラシュート降下では 0）と、それが 10 s を超えたか（真偽値）。``true`` のときは警告を出す（:ref:`sec-lim-aero`）
   * - ``roll_resonance``
     - ロール角速度がピッチの固有振動数を横切ったときだけ書く（トリムの強制がある飛翔で、頂点より前の自由飛行。:ref:`sec-roll-resonance`）。
       最初の交差の時刻 ``t`` [s]、Mach 数 ``mach``、そのときのロール角速度（＝固有振動数）``frequency`` [rad/s]、上昇中の交差の回数 ``crossings`` を持つ。警告も出す（:ref:`sec-config-checks`）
   * - ``drogue_deploy_speed`` / ``drogue_deploy_q``
     - パラシュート（二段開傘ではドローグ）が開いたときの対気速度 [m/s]\ （風に対する速さ）と動圧 [Pa]。``drogue_deploy_time`` と同じく、開傘しなかった飛翔では書かない。
       対気速度が 50 m/s を超えるか動圧が 2 kPa を超えると警告する（:ref:`sec-config-checks`）
   * - ``main_deploy_speed`` / ``main_deploy_q``
     - 二段開傘のメインが開いたときの対気速度 [m/s] と動圧 [Pa]。メインが開かなかった飛翔では書かない
   * - ``earth_model``
     - 地球のモデル ``earth.model``\ （``"flat"`` または ``"ecef"``）
   * - ``wind_model``
     - 風のモデル ``wind.model``\ （``"constant"``、``"power"``、``"log"``、``"profile"``）
   * - ``wind_scale``
     - ``wind.model = "profile"`` のときだけ書く。その飛翔で表の風速に掛けた倍率 :math:`k`\ （CSV の表そのままなら 1。風の格子とモンテカルロのケースは、そのケースの平均風速に合わせた倍率。:ref:`sec-wind-scaling`）
   * - ``wind_at_ref_height``
     - ``wind.model = "profile"`` のときだけ書く。公称の表の風を ``wind.ref_height`` で求めた値で、高さ ``height`` [m]、風速 ``speed`` [m/s]、吹いてくる方位 ``direction_deg`` [deg] を持つ（無風なら方位は 0）。
       保安距離での使い方は :ref:`sec-dispersion-safety` にある
   * - ``max_time``
     - 打ち切り時刻 ``sim.max_time`` [s]
   * - ``config_hash``
     - この結果を計算した設定の識別子（16 桁の 16 進数）。出力先、計算資源（``[output]`` と ``[resources]``）、``[dispersion]`` を除く設定と、入力ファイル（STL、モータ、風のプロファイル、外部の係数表）の内容で決まる。結果が現在の設定で計算されたものかの照合に使い、KML は現在の設定と同じ識別子の結果だけをまとめる（:ref:`sec-kml`）。落下分散の出力にある ``config_hash`` と ``flight_config_hash`` も同じ目的の識別子である（:ref:`sec-file-dispersion`）
   * - ``warnings``
     - 警告の文字列の配列（空のこともある。:ref:`sec-config-checks`）。各文字列は、画面の ``warning[IY-O514]: …`` の形ではなく、コードを角括弧のまま先頭に付けた ``[IY-O514] …`` の形で書く（:ref:`errors`）。設定の読み込み時の警告（``profile`` で使わない ``wind.speed`` と ``wind.direction_deg``、2 回以上指定した ``--set`` のキー）が先頭で、計算中の警告が続く。``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` も同じ順である
   * - ``aero_table_range``
     - 係数表の範囲外の引き出しの集計。全引き出しの回数 ``lookups``、Mach 数が表の最小値未満・最大値超の回数 ``mach_below`` ``mach_above``、迎角または横滑り角が格子の外の回数 ``angle_beyond``、要求された最大の Mach 数 ``max_mach`` と最大の迎角 ``max_abs_alpha_deg`` [deg]（軸対称表では引いた全迎角、一般表では :math:`\alpha` と :math:`\beta` から求めた全迎角 :math:`\arccos(\cos\alpha\cos\beta)`）、範囲外が 1 回でもあったか ``out_of_range``\ （真偽値）、迎角の範囲外がなかったか ``angles_in_range`` と Mach 数の範囲外がなかったか ``machs_in_range``\ （真偽値。どちらかが ``false`` のときは結果を使えず、終了ステータス 3 で終わる。:ref:`sec-exit-status`）

``sim`` の実行が失敗したとき（積分の発散、``launch.elevation_deg`` が範囲外、ランチャを離れないなど）は、
``summary.json`` を失敗の印 ``{"failed": true, "error": "…"}`` だけで書き、``error`` にエラーの文を入れる。``trajectory.csv`` は書かない。``ignisyeet.kml`` があれば、飛翔の層を除いて書き直す（:ref:`sec-kml`）。
``python/plot.py`` は、この印があるとき飛翔経路を描かずにエラーで終わる。

.. _sec-file-trajectory:

``trajectory.csv``
~~~~~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,beta_deg,roll_rate,cl,cy

時刻歴である。座標は射点を原点とする ENU（東・北・上）で、機体座標は機首方向を x 軸とする。

.. list-table::
   :header-rows: 1
   :widths: 20 12 68

   * - 列
     - 単位
     - 内容
   * - ``t``
     - s
     - 点火からの時刻
   * - ``phase``
     - 
     - ``rail``（ランチャ上）、``free``（自由飛行）、``parachute``（パラシュート降下）のいずれか
   * - ``east`` ``north`` ``up``
     - m
     - 射点からの位置。``up`` は上向きが正
   * - ``lat`` ``lon``
     - deg
     - 緯度と経度
   * - ``alt``
     - m
     - 標高（射点では ``launch.altitude``）
   * - ``vel_e`` ``vel_n`` ``vel_u``
     - m/s
     - 地面に対する速度の東・北・上成分。``vel_u`` は上昇が正
   * - ``airspeed``
     - m/s
     - 風に対する速さ
   * - ``mach``
     - 
     - Mach 数
   * - ``alpha_deg``
     - deg
     - 全迎角 :math:`\alpha_t`
   * - ``thrust``
     - N
     - 推力
   * - ``mass``
     - kg
     - その時刻の質量
   * - ``x_cg``
     - m
     - 重心。ノーズ先端から後方へ測る
   * - ``x_cp``
     - m
     - 圧力中心。ノーズ先端から後方へ測る。その状態が飛んでいる迎角での値で、一般表では等価な圧力中心（:eq:`eq-xcp-eff`）
   * - ``stability_cal``
     - cal
     - :math:`(x_{cp}-x_{cg})/d`。その状態の迎角での表示で、正が重心より後ろに圧力中心がある向きである。一般表ではこの値は ``x_cp`` から求める。迎角が大きいと圧力中心が前に動くので、ランチャ離脱の直後や頂点付近では小さくなる。静的な安定の判定と ``summary.json`` の余裕は迎角 0 の極限の小迎角の余裕で、この列とは違う（:ref:`sec-static-margin-general`）
   * - ``dyn_pressure``
     - Pa
     - 動圧
   * - ``cn`` ``ca``
     - 
     - 法線力係数と軸力係数。``ca`` は抗力の向きを正とする。一般表の ``cn`` は :math:`C_Y` と :math:`C_Z` の合成
   * - ``pitch_deg``
     - deg
     - 機体軸の水平面に対する仰角。上向きが正で、鉛直上向きが 90
   * - ``heading_deg``
     - deg
     - 機体軸の水平成分の方位。北が 0 で東回りに 0 以上 360 未満
   * - ``wx`` ``wy`` ``wz``
     - rad/s
     - 機体座標の角速度成分（右手系の正）。``earth.model = "ecef"`` では ECEF に対する角速度
   * - ``beta_deg``
     - deg
     - 横滑り角 :math:`\beta`
   * - ``roll_rate``
     - rad/s
     - ロール角速度 :math:`\omega_x`（``wx`` と同じ値）
   * - ``cl``
     - 
     - 適用したロールの係数 :math:`C_l^{\mathrm{tot}}`\ （:eq:`eq-cl-total`）
   * - ``cy``
     - 
     - 横力係数（機体座標の y）。軸対称表では 0

``dispersion.csv``
~~~~~~~~~~~~~~~~~~

``mode = "wind_grid"`` が書く。見出しは次のとおりである。

.. code-block:: text

   descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,landed,error,stable,broadside_time

風の各ケースの着地点（ENU と緯度経度）、頂点、飛行時間などである。
``wind_speed`` と ``wind_direction_deg`` は格子の値で、``constant`` ``power`` ``log`` では基準高度の風速と風向、``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` に対する値である（:ref:`sec-wind-scaling`）。
``landing_speed`` は着地の瞬間の地面に対する速さである（:ref:`sec-file-summary`）。
末尾の ``landed`` は、そのケースが ``sim.max_time`` までに着地したかを ``true`` または ``false`` で示す。
着地しなかったケースでは、着地に関する列（``landing_east`` から ``landing_speed`` まで）は ``NaN`` になる。
列 ``error`` は、積分がエラーで止まったケースのエラーの文である（改行は空白に替え、コンマや引用符を含むときは引用符で囲む）。エラーのないケースでは空である。
エラーで止まったケースは、``landed = false`` で、``wind_speed`` と ``wind_direction_deg`` 以外の数値の列が ``NaN`` になる（:ref:`sec-wind-grid-failed`）。
列 ``stable`` は、そのケースが静的に安定か（``summary.json`` の ``stable`` と同じ判定で、安定なら ``true``、不安定なら ``false``）である。判定できないケースと、エラーで止まったケースでは空である（:ref:`sec-dispersion-unstable`）。
最後の列 ``broadside_time`` は、``summary.json`` と同じ、頂点のあとの弾道落下で全迎角が 60° から 120° の間で飛んだ時間 [s]（小数 2 桁）である。パラシュート降下の行では 0、エラーで止まったケースでは空である。10 を超える弾道落下の行が、横向きに落ちたケースである（:ref:`sec-dispersion-broadside`）。
一般表では、``min_stability_cal`` は各ステップのピッチとヨーの余裕の小さいほうの最小値（:ref:`sec-static-margin-general`）である。

.. _sec-file-dispersion:

``dispersion_meta.json``
~~~~~~~~~~~~~~~~~~~~~~~~

``mode = "wind_grid"`` が ``dispersion.csv`` と並べて書く。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``config_hash``
     - 風の格子の結果を計算した設定の識別子（``mode`` と ``[dispersion.monte_carlo]`` を除く設定で決まる）。用途は ``summary.json`` の ``config_hash`` と同じである（:ref:`sec-file-summary`）
   * - ``flight_config_hash``
     - 飛翔の設定の識別子。``summary.json`` の ``config_hash`` と同じ値で、``sim`` の結果と同じ設定かの照合に使う
   * - ``cases``
     - ケース数
   * - ``failed`` / ``errored``
     - 着地しなかったケースの数と、そのうち積分がエラーで止まったケースの数
   * - ``warnings``
     - 警告の文字列の配列
   * - ``aero_table_range``
     - 係数表の範囲外の引き出しの集計。項目は :ref:`sec-file-summary` と同じで、エラーで止まったケースは数えない
   * - ``static_margin_min_cal`` / ``stable``
     - 公称の飛翔の最小の静安定余裕 [cal] と、それが 0 以上かを示す真偽値（公称の飛翔は、設定した風での弾道落下）。公称の飛翔が失敗して余裕が求まらないときと、静的な安定を判定できないときは、``stable`` が ``null`` である
   * - ``unstable`` / ``unstable_min_static_margin_cal``
     - 降下モード（``ballistic`` と ``parachute``）ごとの、静的に不安定なケースの数と、そのモードのケースの最小の静安定余裕 [cal]。余裕が求まるケースがなければ ``null``
   * - ``stability_unknown_runs`` / ``stability_unknown``
     - 降下モードごとの、静的な安定を判定できないケースの数と、公称の飛翔が判定できないときの理由（判定できたときは ``null``）
   * - ``broadside_runs``
     - 横向きに落ちた（``broadside_time`` が 10 s を超えた）ケースの数。弾道落下だけを数え、``{"ballistic": 1}`` の形
   * - ``wind_model``
     - 公称の風のモデル
   * - ``wind_speed`` / ``wind_direction_deg``
     - 公称の基準風速と基準風向。``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`\ （``profile_mean_speed`` と ``profile_mean_direction_deg`` を指定したときはその値）
   * - ``wind_scale`` / ``wind_at_ref_height``
     - ``profile`` のときだけ書く。公称の表の倍率（CSV の風速に掛ける倍率。格子の風速はここからさらに拡大する。:ref:`sec-wind-scaling`）と、``wind.ref_height`` での公称の表の風（``summary.json`` と同じ）

``stable`` が ``false`` か ``null`` のとき（公称の飛翔が失敗した場合を除く）、``unstable`` または ``stability_unknown_runs`` が 1 以上のモードがあるとき、迎角または Mach 数の範囲外の引き出しがあったとき（``angles_in_range`` または ``machs_in_range`` が ``false``）、``dispersion`` は終了ステータス 3 で終わる（:ref:`sec-exit-status`、:ref:`sec-dispersion-unstable`）。

``dispersion_mc.csv``
~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く、サンプルごと・降下モードごとの 1 行である。見出しは次のとおりである。

.. code-block:: text

   sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,roll_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,thrust_misalignment_y_deg,thrust_misalignment_z_deg,thrust_offset_y,thrust_offset_z,fin_cant_deg,cg_lateral_y,cg_lateral_z,cl_scale,clp_scale,lag_scale,magnus_scale,cmq_scale,stable,broadside_time

``status`` は ``ok`` または ``failed`` で、失敗した行は結果の列が空欄になる。``sim.max_time`` までに着地しなかったサンプルも ``failed`` である。
``thrust_scale`` から ``parachute_cd_s_scale`` までは引いたばらつきで、相対のものは割合、``wind_speed_delta`` と ``wind_direction_deg_delta`` は風速・風向のずれである。
``elevation_deg`` と ``azimuth_deg`` は、引いた値そのものではなく、実際に飛ばしたレールの射角と射方位の公称値からの変化 [deg]、``roll_deg`` は機体のロール角の公称値からの変化 [deg] である（射方位とロール角は :math:`(-180^\circ,180^\circ]` に折り返す。:ref:`sec-mc-rail`）。
``wind_speed`` と ``wind_direction_deg`` はそのサンプルの基準風速・風向（``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` を拡大・回転した値）、続く列は着地点（ENU と緯度経度）、着地距離、着地時刻、着地速度、頂点、頂点時刻、最大 Mach 数、ランチャ離脱速度、最小静安定余裕である。
続く 9 列（``thrust_misalignment_y_deg`` から ``clp_scale``）は、ロールと非対称について引いた値、その次の 3 列（``lag_scale``、``magnus_scale``、``cmq_scale``）は空力の遅れ、Magnus 力、減衰の和について引いた割合である（:ref:`sec-mc`）。
列 ``stable`` は、そのサンプルが静的に安定か（``summary.json`` の ``stable`` と同じ判定で、安定なら ``true``、不安定なら ``false``）で、判定できないサンプルと失敗したサンプルでは空である（:ref:`sec-dispersion-unstable`）。
最後の列 ``broadside_time`` は ``dispersion.csv`` と同じで、失敗したサンプルでは空である。

``dispersion_summary.json``
~~~~~~~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く。全体の項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``samples`` / ``seed``
     - サンプル数と乱数の種
   * - ``truncate_sigma``
     - 標準正規乱数を切る範囲（``dispersion.monte_carlo.truncate_sigma`` の値。切らないときは ``null``。:ref:`sec-mc`）
   * - ``descents``
     - 降下モードごとの配列（下表）
   * - ``config_hash`` / ``flight_config_hash``
     - 識別子。``config_hash`` はモンテカルロの結果を計算した設定（``mode`` と風の格子の設定を除く）で決まる。``flight_config_hash`` は ``summary.json`` の ``config_hash`` と同じ値である
   * - ``warnings`` / ``aero_table_range``
     - 警告の配列と、係数表の範囲外の引き出しの集計（項目は :ref:`sec-file-summary` と同じ。失敗したサンプルは数えない）
   * - ``static_margin_min_cal`` / ``stable``
     - 公称の飛翔の最小の静安定余裕 [cal] とその判定。``dispersion_meta.json`` と同じで、公称の飛翔が失敗したときは ``null``
   * - ``unstable`` / ``unstable_min_static_margin_cal``
     - 降下モードごとの、静的に不安定なサンプルの数と最小の静安定余裕。``dispersion_meta.json`` と同じで、失敗したサンプルは余裕を持たないので数に入らない
   * - ``stability_unknown_runs`` / ``stability_unknown`` / ``broadside_runs``
     - ``dispersion_meta.json`` と同じ
   * - ``wind_model`` / ``wind_speed`` / ``wind_direction_deg`` / ``wind_scale`` / ``wind_at_ref_height``
     - 公称の風。``dispersion_meta.json`` と同じで、モンテカルロの風速と風向の揺らぎはこの公称の風の周りに引く

``descents`` の各要素の項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``descent``
     - 降下モード
   * - ``n`` / ``failed`` / ``not_landed``
     - 着地したサンプル数、失敗数、そのうち着地しなかった数
   * - 着地点の平均と共分散行列
     - 着地点の統計
   * - 誤差楕円
     - :math:`1\sigma` と :math:`3\sigma` の長半径、短半径、長軸の方位
   * - ``max_landing_distance``
     - 最大着地距離とそのサンプル番号
   * - ``landing_distance_p50`` ``_p90`` ``_p99`` ``_p999``
     - 着地距離のパーセンタイル。着地したサンプルについて線形補間で求める。P99.9 は、着地したサンプルが 1000 未満の降下モードでは ``null``（:ref:`P99.9 の条件 <sec-mc-percentile>`）
   * - 頂点高度
     - 平均と標準偏差（:ref:`sec-mc`）

失敗したサンプルは、どの統計にも入らない。``failed`` が 1 以上のとき、``complete`` は ``false`` で、統計は着地したサンプルだけを表し、真のばらつきはこれより大きいか不明である。
同じとき ``max_landing_distance_is_lower_bound`` が ``true`` になり、最大着地距離は下限である。
着地点の平均は着地したサンプルが 1 つ以上、標準偏差・共分散・誤差楕円・頂点高度の標準偏差は 2 つ以上のとき求まる。サンプルが足りない統計は ``null`` である
（着地したサンプルが一つもなければ、統計と距離の項目はすべて ``null``）。
