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

``sim`` が書く、1 回の飛翔の要約である。
降下モード ``descent``、基準風 ``wind_speed`` と ``wind_direction_deg``\ （``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`。:ref:`sec-wind-profile`）、ランチャ離脱の時刻 ``rail_exit_time`` と速度 ``rail_exit_speed``、燃焼終了時刻 ``burnout_time``\ （推力曲線の終わりの時刻。飛翔がそれより前に終わったときは ``null``）、
最大速度 ``max_speed``、最大 Mach 数 ``max_mach``、最大動圧 ``max_dyn_pressure``\ （積分のステップの始端ごとの状態と、着地の状態のうちの最大）、頂点の時刻 ``apogee_time`` と高度 ``apogee``\ （射点からの高さ）と位置 ``apogee_east`` ``apogee_north``、
最小静安定余裕 ``min_stability_cal`` とランチャ離脱時の値 ``stability_at_rail_exit``\ （どちらも迎角 0 の極限の小迎角の余裕で、その状態が飛んでいる迎角は入らない。:ref:`sec-static-margin-general`。``min_stability_cal`` は、ランチャ離脱の時刻（自由飛行はここから始まる）から頂点までのうち対気速度が 10 m/s を超える間に、積分のステップの始端ごとに測った最小値である。
離脱時の対気速度が 10 m/s を超えるときは、ランチャ離脱の時刻の値を含む。頂点付近の遅い部分と降下は含まない。
該当する時刻がなければ（頂点までに対気速度が 10 m/s を超えない飛翔など）``null`` である。``stability_at_rail_exit`` は対気速度によらず求める。軸対称表では小さな迎角の圧力中心から求めた :math:`(x_{cp}(\delta)-x_{cg})/d`\ （:eq:`eq-margin-axisym`）、一般表では各ステップのピッチとヨーの余裕の小さいほう）、
ランチャ離脱の値と着地の値は、ステップ内の補間（位置は三次エルミート補間、速度はその時間微分、残りは両端の線形補間）で求めた状態の値である（:ref:`sec-rail`、:ref:`sec-landing`）、
着地の時刻 ``landing_time``、位置 ``landing_east`` ``landing_north`` ``landing_lat`` ``landing_lon``、射点からの距離 ``landing_distance``、着地速度 ``landing_speed`` を持つ。
``landing_speed`` は着地の瞬間の **地面に対する速さ** で、水平方向の流され（風による移動）の速さを含む。降下率（鉛直の速さ）ではないので、パラシュート降下で風が強いと、降下率より大きい。
これに加えて、次の項目を書く。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
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
     - 静的に安定なら ``true``、不安定（求まった余裕のどれかが負）なら ``false``、判定できない（求まらない余裕がある、またはどの余裕も求めなかった）なら ``null``。
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
       風速の上限 :math:`V_{\mathrm{lim}}` を測る高さが ``wind.ref_height`` なら、保安距離の手順 1 の格子の上限は :math:`v_{\max}=W\,V_{\mathrm{lim}}/` ``speed``\ （:math:`W` は ``wind_speed``）になる（:ref:`sec-dispersion-safety`）。
       ``sim`` と ``dispersion`` は、端末にも ``profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)`` の形の行を出す（例の表の値）
   * - ``max_time``
     - 打ち切り時刻 ``sim.max_time`` [s]
   * - ``config_hash``
     - この結果を計算した設定の識別子（16 桁の 16 進数）。出力先、計算資源（``[output]`` と ``[resources]``）、``[dispersion]`` を除く設定と、入力ファイル（STL、モータ、風のプロファイル、外部の係数表）の内容で決まる。同じ設定で計算した結果かの照合に使う。KML は、現在の設定と同じ設定で計算された結果だけをまとめる（:ref:`sec-kml`）
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

時刻歴で、位置（``east`` ``north`` ``up``、``lat`` ``lon`` ``alt``）、速度、``airspeed``、``mach``、``thrust``、``mass``、``x_cg``、``x_cp``、``stability_cal``、``dyn_pressure``、姿勢と角速度を持つ。
``x_cp`` と ``stability_cal`` は、その状態が飛んでいる迎角での圧力中心と :math:`(x_{cp}-x_{cg})/d` [cal] の表示である。迎角が大きいと圧力中心が前に動くので、ランチャ離脱の直後や頂点付近では小さくなる。
静的な安定の判定と ``summary.json`` の余裕は、迎角 0 の極限の小迎角の余裕で、この列とは違う（:ref:`sec-static-margin-general`）。
``phase`` は ``rail``、``free``、``parachute`` のいずれかである。
``alpha_deg`` は全迎角 :math:`\alpha_t`、``x_cp`` は一般表では等価な圧力中心（:eq:`eq-xcp-eff`）で、一般表の ``stability_cal`` はこの値から求める。
末尾の 4 列は、``beta_deg``\ （横滑り角 :math:`\beta` [deg]）、``roll_rate``\ （ロール角速度 :math:`\omega_x` [rad/s]）、
``cl``\ （ロールの係数 :math:`C_l^{\mathrm{tot}}`、:eq:`eq-cl-total`）、``cy``\ （一般表の :math:`C_Y`。軸対称表では 0）である。

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

``mode = "wind_grid"`` が ``dispersion.csv`` と並べて書く。項目は、設定の識別子 ``config_hash``\ （風の格子について、``mode`` と ``[dispersion.monte_carlo]`` を除いた設定で決まる）、
飛翔の設定の識別子 ``flight_config_hash``\ （``summary.json`` の ``config_hash`` と同じ決め方で、``sim`` の結果と同じ設定かの照合に使う）、
ケース数 ``cases``、着地しなかったケースの数 ``failed``、そのうち積分がエラーで止まったケースの数 ``errored``、警告の文字列の配列 ``warnings``、係数表の範囲外の引き出しの集計 ``aero_table_range``\ （項目は :ref:`sec-file-summary` と同じ。エラーで止まったケースは数えない）、
公称の飛翔の最小の静安定余裕 ``static_margin_min_cal`` [cal] と、それが 0 以上かを示す ``stable``\ （真偽値）、
降下モード（``ballistic`` と ``parachute``）ごとの静的に不安定なケースの数 ``unstable`` と、そのモードのケースの最小の静安定余裕 ``unstable_min_static_margin_cal`` [cal]\ （余裕が求まるケースがなければ ``null``）、
降下モードごとの静的な安定を判定できないケースの数 ``stability_unknown_runs`` と、公称の飛翔が判定できないときの理由 ``stability_unknown``\ （判定できたときは ``null``）、
横向きに落ちた（``broadside_time`` が 10 s を超えた）弾道落下のケースの数 ``broadside_runs``\ （``{"ballistic": 1}`` の形。弾道落下だけを数える）、
公称の風のモデル ``wind_model``、公称の基準風速 ``wind_speed`` と基準風向 ``wind_direction_deg``\ （``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`。``profile_mean_speed`` と ``profile_mean_direction_deg`` を指定したときはその値）、
``profile`` のときだけ公称の表の倍率 ``wind_scale``\ （CSV の風速に掛ける倍率。格子の風速はここからさらに拡大する。:ref:`sec-wind-scaling`）と ``wind.ref_height`` での公称の表の風 ``wind_at_ref_height``\ （``summary.json`` と同じ）である。
公称の飛翔は、設定した風での弾道落下である。公称の飛翔が失敗して余裕が求まらないときは、``static_margin_min_cal`` と ``stable`` は ``null`` である。公称の飛翔の静的な安定を判定できないときも ``stable`` は ``null`` である。
``stable`` が ``false`` か ``null`` のとき（公称の飛翔が失敗した場合を除く）、``unstable`` または ``stability_unknown_runs`` が 1 以上のモードがあるとき、迎角または Mach 数の範囲外の引き出しがあったとき（``aero_table_range`` の ``angles_in_range`` または ``machs_in_range`` が ``false``）、``dispersion`` は終了ステータス 3 で終わる（:ref:`sec-exit-status`、:ref:`sec-dispersion-unstable`）。
KML は、このファイルの設定が現在の設定と一致するときだけ ``dispersion.csv`` を取り込む。

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

``mode = "monte_carlo"`` が書く。全体の項目はサンプル数 ``samples``、乱数の種 ``seed``、標準正規乱数を切る範囲 ``truncate_sigma``\ （``dispersion.monte_carlo.truncate_sigma`` の値。切らないときは ``null``。:ref:`sec-mc`）、降下モードごとの配列 ``descents``、設定の識別子 ``config_hash``\ （モンテカルロについて、``mode`` と風の格子の設定を除いた設定で決まる）、飛翔の設定の識別子 ``flight_config_hash``\ （``summary.json`` の ``config_hash`` と同じ決め方）、警告の配列 ``warnings``、係数表の範囲外の引き出しの集計 ``aero_table_range``\ （項目は :ref:`sec-file-summary` と同じ。失敗したサンプルは数えない）、
公称の飛翔の最小の静安定余裕 ``static_margin_min_cal`` [cal] と、それが 0 以上かを示す ``stable``\ （風の格子の ``dispersion_meta.json`` と同じ。公称の飛翔が失敗したときは ``null``）、
降下モードごとの静的に不安定なサンプルの数 ``unstable`` と最小の静安定余裕 ``unstable_min_static_margin_cal``\ （風の格子の ``dispersion_meta.json`` と同じ。失敗したサンプルは余裕を持たないので、数に入らない）、
``stability_unknown_runs`` と ``stability_unknown``、横向きに落ちた弾道落下のサンプルの数 ``broadside_runs``、公称の風 ``wind_model``、``wind_speed``、``wind_direction_deg``、``wind_scale``、``wind_at_ref_height``\ （いずれも風の格子の ``dispersion_meta.json`` と同じ）である。
モンテカルロの風速と風向の揺らぎは、この公称の風の周りに引く。
``descents`` の各要素は、降下モード ``descent``、着地したサンプル数 ``n``、失敗数 ``failed``、そのうち着地しなかった数 ``not_landed``、着地点の平均と共分散行列、
:math:`1\sigma` と :math:`3\sigma` の誤差楕円（長半径、短半径、長軸の方位）、最大着地距離 ``max_landing_distance`` とそのサンプル番号、
着地距離のパーセンタイル ``landing_distance_p50`` ``landing_distance_p90`` ``landing_distance_p99`` ``landing_distance_p999``\ （P99.9。着地したサンプルについて、線形補間で求める。着地したサンプルが 1000 未満の降下モードでは ``null``。:ref:`P99.9 の条件 <sec-mc-percentile>`）、頂点高度の平均と標準偏差を持つ（:ref:`sec-mc`）。
失敗したサンプルは、どの統計にも入らない。``failed`` が 1 以上のとき、``complete`` は ``false`` で、統計は着地したサンプルだけを表し、真のばらつきはこれより大きいか不明である。
同じとき ``max_landing_distance_is_lower_bound`` が ``true`` になり、最大着地距離は下限である。
着地点の平均は着地したサンプルが 1 つ以上、標準偏差・共分散・誤差楕円・頂点高度の標準偏差は 2 つ以上のとき求まる。サンプルが足りない統計は ``null`` である
（着地したサンプルが一つもなければ、統計と距離の項目はすべて ``null``）。
