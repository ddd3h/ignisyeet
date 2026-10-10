.. _page-config:

.. _page-reference-config:

設定ファイル
============

シミュレーションの入力は `TOML <https://toml.io/ja/v1.0.0>`_ 形式の設定ファイル 1 つにまとめる。
ファイルは機体、推進、空力、環境、積分、落下分散といった表（``[wind]`` など）に分かれ、
並びは ``examples/sample.toml`` と同じである。この章は各表のキーを、型と既定値、意味と値の範囲とともに引けるように並べる。
相対パスは設定ファイルが置かれたディレクトリを基準に解釈する。ただし ``--out`` で与える出力先は、実行時のカレントディレクトリを基準にする。

どのキーも、コマンドラインで ``--set 表.キー=値`` として上書きできる（:ref:`sec-cli-overrides`）。
設定ファイルは読み込み時に検査し、不正な値はキーの名前を示してエラーにする。有限でない値（``nan`` や ``inf``）はどの数値のキーでも受け付けない。

長さの単位は m、角度の単位は deg である。キーごとに単位が違うものは、意味の欄に示す。
機体上の位置はすべてノーズ先端から後方へ測る。

.. _page-reference-config-general:

.. _cfg-output:

``[output]``
------------

結果ファイルの出力先と、KML の有無を指定する。
出力先は ``--out`` でも与えられ、そのときはコマンドラインの値が優先する。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-output-dir:

       ``output.dir``
          パス、``"out"``
     - 結果ファイルを書き出すディレクトリ。相対パスは設定ファイルのあるディレクトリを基準にする。``--out`` で上書きできる（:ref:`sec-cli-overrides`）。
   * - .. _cfg-output-kml:

       ``output.kml``
          真偽値、``true``
     - Google Earth 用の ``ignisyeet.kml`` を書き出すか（:ref:`sec-kml`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-resources:

``[resources]``
---------------

計算資源の上限を指定する。すべて省略でき、結果は変えない。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-resources-threads:

       ``resources.threads``
          整数、``0``
     - 全処理で使う CPU スレッド数の上限。``0`` は使用可能な全スレッド。使用可能数を超える値は警告して使用可能数に切り詰める。落下分散、モンテカルロ、パネル法の行列計算、CFD の MPI ランク数に効く。0 以上。
   * - .. _cfg-resources-memory_gb:

       ``resources.memory_gb``
          数値、``0``
     - 重い処理のメモリ予算 [GB]。``0`` はこのプロセスが使えるメモリ（Linux では ``MemAvailable``、cgroup の上限、``ulimit -v`` の上限の最小。macOS では物理メモリ）。
       パネル法は、密行列に必要な量（パネル数 :math:`n` に対して :math:`8n^2+64n` バイト）が予算を超えると、計算の前にエラーで止まる。
       CFD は、予算に収まるよう同時実行ケース数を減らす。1 ケースの見積もりが予算を超えるときは、メッシュ生成の後、計算の前にエラーで止まる。0 以上（負の値はエラー）（:ref:`sec-cfd-resources`）。
   * - .. _cfg-resources-nice:

       ``resources.nice``
          整数、``0``
     - プロセスの優先度を下げる値。SU2 や ``mpirun`` の子プロセスにも引き継がれる。Unix 以外のプラットフォームでは警告して無視する。0 以上 19 以下。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _sec-config-rocket:

.. _cfg-rocket:

``[rocket]``
------------

機体の CAD データ（STL、または STEP、IGES、BREP のファイル）と質量特性、表面とフィンの仕様を指定する。
STL の読み方と形状の抽出は :ref:`page-theory-geometry` にある。
質量、重心、慣性モーメントは推進剤を除いた乾燥状態の値で与え、燃焼中の変化はモータの推進剤質量から決まる。
慣性テンソル、``cg_lateral_dry``、``misalignment_offset`` は機体座標（:math:`x_b` がノーズ方向）で与える（:ref:`sec-body-basis`）。
「必須」のキーは省略できない。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-rocket-stl:

       ``rocket.stl``
          パス、``rocket.cad`` とどちらか一方が必須
     - 機体の STL ファイル（ASCII とバイナリのどちらも読める）。``rocket.cad`` と同時には指定できない（:ref:`IY-C064`）。
   * - .. _cfg-rocket-stl_scale:

       ``rocket.stl_scale``
          数値、``1.0``
     - STL の長さの単位を m に直す係数。mm の STL は ``0.001``。掛けた後の機体の全長は 0.1 m 以上 20 m 以下。正。``rocket.cad`` と同時には ``1.0`` 以外にできない（CAD の単位は ``rocket.cad_unit`` で決める。:ref:`IY-C065`）。
   * - .. _cfg-rocket-cad:

       ``rocket.cad``
          パス、``rocket.stl`` とどちらか一方が必須
     - 機体の CAD ファイル。STEP（``.step`` ``.stp``）、IGES（``.iges`` ``.igs``）、BREP（``.brep`` ``.brp``）を読める。STL に書き出し直さずに、形状の抽出から飛翔の計算までの入力にできる。相対パスは設定ファイルのディレクトリを基準にする。``rocket.stl`` と同時には指定できない（:ref:`IY-C064`）。読むには CFD 用ツールの gmsh（Python）が要る（:ref:`IY-G017`）。``aero.cfd.surface = "step"`` で ``step_file`` を省くと、このファイルが CFD の壁面になる（:ref:`page-howto-own-rocket`）。
   * - .. _cfg-rocket-cad_unit:

       ``rocket.cad_unit``
          文字列、``"auto"``
     - ``rocket.cad`` の長さの単位。``"auto"`` はファイルが宣言する単位（BREP は m）で読む。単位を宣言していない STEP と IGES では ``"auto"`` を指定できない（:ref:`IY-G018`）。``rocket.stl`` と同時には指定できない。選べる値は ``"auto"``、``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``。
   * - .. _cfg-rocket-nose_direction:

       ``rocket.nose_direction``
          文字列、``"auto"``
     - STL または CAD ファイルの座標でノーズが向く方向。``"auto"`` は形状から判定する。選べる値は ``"auto"``、``"+x"``、``"-x"``、``"+y"``、``"-y"``、``"+z"``、``"-z"``。
   * - .. _cfg-rocket-dry_mass:

       ``rocket.dry_mass``
          数値、必須
     - 推進剤を除いた質量 [kg]。空のモータケースを含む。正。
   * - .. _cfg-rocket-cg_dry:

       ``rocket.cg_dry``
          数値、必須
     - 推進剤を除いた重心のノーズ先端からの位置 [m]。0 以上、機体の全長以下。
   * - .. _cfg-rocket-ixx_dry:

       ``rocket.ixx_dry``
          数値、必須
     - 推進剤を除いたロール慣性モーメント [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。正（:ref:`sec-usage-inertia-estimate`）。
   * - .. _cfg-rocket-iyy_dry:

       ``rocket.iyy_dry``
          数値、必須
     - 推進剤を除いたピッチ慣性モーメント [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。正（:ref:`sec-usage-inertia-estimate`）。
   * - .. _cfg-rocket-izz_dry:

       ``rocket.izz_dry``
          数値、``iyy_dry``
     - 推進剤を除いたヨー慣性モーメント [kg m\ :sup:`2`]。省略すると ``iyy_dry`` に等しい。正。
   * - .. _cfg-rocket-ixy_dry:

       ``rocket.ixy_dry``
          数値、``0``
     - 慣性テンソルの非対角要素 :math:`I_{xy}=-\int xy\,\mathrm{d}m` [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。慣性テンソルは正定値でなければならない。CAD が慣性乗積を :math:`+\int xy\,\mathrm{d}m` で表示するときは符号を反転して与える。有限（:ref:`sec-inertia-tensor`）。
   * - .. _cfg-rocket-ixz_dry:

       ``rocket.ixz_dry``
          数値、``0``
     - 慣性テンソルの非対角要素 :math:`I_{xz}=-\int xz\,\mathrm{d}m` [kg m\ :sup:`2`]。符号の規約は ``ixy_dry`` と同じ。有限（:ref:`sec-inertia-tensor`）。
   * - .. _cfg-rocket-iyz_dry:

       ``rocket.iyz_dry``
          数値、``0``
     - 慣性テンソルの非対角要素 :math:`I_{yz}=-\int yz\,\mathrm{d}m` [kg m\ :sup:`2`]。符号の規約は ``ixy_dry`` と同じ。有限（:ref:`sec-inertia-tensor`）。
   * - .. _cfg-rocket-cg_lateral_dry:

       ``rocket.cg_lateral_dry``
          数値の配列（2 要素）、``[0, 0]``
     - 乾燥重心の機軸からの横ずれ ``[y, z]`` [m]（機体座標）。有限（:ref:`sec-body-basis`）。
   * - .. _cfg-rocket-fin_cant_deg:

       ``rocket.fin_cant_deg``
          数値、``0``
     - フィンのカント [deg]。正のカントは :math:`+x_b` まわりの正のロールモーメントを生む。``fin_override.cant_deg`` があればそちらを使う。有限。
   * - .. _cfg-rocket-roughness:

       ``rocket.roughness``
          数値、``60e-6``
     - 表面の等価砂粒粗さ [m]。摩擦抗力に使う。0 以上。
   * - .. _cfg-rocket-fin_le:

       ``rocket.fin_le``
          文字列、``"rounded"``
     - フィンの前縁形状。丸め、または鋭角。選べる値は ``"rounded"``、``"sharp"``。
   * - .. _cfg-rocket-fin_te:

       ``rocket.fin_te``
          文字列、``"square"``
     - フィンの後縁形状。切り落とし（底面抗力あり）、またはテーパ。選べる値は ``"square"``、``"tapered"``。
   * - .. _cfg-rocket-extra_cd:

       ``rocket.extra_cd``
          数値、``0.0``
     - ランチャラグなど付加物の抗力係数。0 以上。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-rocket-fin_override:

``[rocket.fin_override]``
-------------------------

CAD モデルから検出したフィンの寸法を上書きする。書いたキーだけが置き換わり、省略したキーは検出した値のままである。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-rocket-fin_override-count:

       ``rocket.fin_override.count``
          整数、検出値
     - フィンの枚数。0 以上。
   * - .. _cfg-rocket-fin_override-root_chord:

       ``rocket.fin_override.root_chord``
          数値、検出値
     - フィン根元の翼弦長 [m]。0 以上。
   * - .. _cfg-rocket-fin_override-tip_chord:

       ``rocket.fin_override.tip_chord``
          数値、検出値
     - フィン先端の翼弦長 [m]。0 以上。
   * - .. _cfg-rocket-fin_override-span:

       ``rocket.fin_override.span``
          数値、検出値
     - フィンのスパン [m]（胴体表面から先端まで）。0 以上。
   * - .. _cfg-rocket-fin_override-sweep:

       ``rocket.fin_override.sweep``
          数値、検出値
     - 前縁の後退の軸方向長さ [m]。有限。
   * - .. _cfg-rocket-fin_override-thickness:

       ``rocket.fin_override.thickness``
          数値、検出値
     - フィンの厚さ [m]。``aero.method = "panel"`` と、``aero.method = "cfd"`` で ``aero.cfd.surface = "panel_mesh"`` のときは正。部品積み上げ法では抗力にだけ使う。0 以上。
   * - .. _cfg-rocket-fin_override-x_le_root:

       ``rocket.fin_override.x_le_root``
          数値、検出値
     - フィン根元の前縁のノーズ先端からの位置 [m]。有限。
   * - .. _cfg-rocket-fin_override-cant_deg:

       ``rocket.fin_override.cant_deg``
          数値、``rocket.fin_cant_deg``
     - フィンのカント [deg]。``rocket.fin_cant_deg`` の代わりに使う。有限。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-motor:

``[motor]``
-----------

モータの推力曲線と取り付けを指定する。推力、質量の変化、推力線の扱いは :ref:`page-theory-propulsion` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-motor-eng:

       ``motor.eng``
          パス、必須
     - RASP 形式（``.eng``）の推力曲線ファイル（:ref:`sec-propulsion-eng`）。
   * - .. _cfg-motor-aft_x:

       ``motor.aft_x``
          数値、機体の全長
     - モータ後端のノーズ先端からの位置 [m]。0 以上。モータの長さ以上、機体の全長以下（1 mm の許容がある）。
   * - .. _cfg-motor-nozzle_exit_diameter:

       ``motor.nozzle_exit_diameter``
          数値、``0.0``
     - ノズル出口の直径 [m]。燃焼中はこの面積だけ底面抗力が減る。0 以上。
   * - .. _cfg-motor-propellant_mass:

       ``motor.propellant_mass``
          数値、``.eng`` の値
     - 推進剤質量 [kg]。``.eng`` の値を上書きする。0 以上。
   * - .. _cfg-motor-misalignment_deg:

       ``motor.misalignment_deg``
          数値、``0``
     - 推力の方向と機軸のなす角 :math:`\varepsilon` [deg]。有限（:ref:`sec-thrust-line`）。
   * - .. _cfg-motor-misalignment_azimuth_deg:

       ``motor.misalignment_azimuth_deg``
          数値、``0``
     - 推力の傾きの向き :math:`\phi` [deg]。:math:`+y_b` から :math:`+z_b` へ測る。有限（:ref:`sec-thrust-line`）。
   * - .. _cfg-motor-misalignment_offset:

       ``motor.misalignment_offset``
          数値の配列（2 要素）、``[0, 0]``
     - ノズル出口の点 ``[y, z]`` [m]（機体座標）。推力線はこの点を通る。有限（:ref:`sec-thrust-line`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

``motor.aft_x`` はモータの長さ（``.eng`` の長さ）以上で、機体の全長以下でなければならない（1 mm の許容がある）。

.. _cfg-launch:

``[launch]``
------------

射点とランチャの条件を指定する。座標系と初期姿勢の定義は :ref:`page-theory-coordinates` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-launch-latitude:

       ``launch.latitude``
          数値、必須
     - 射点の緯度 [deg]（WGS84）。-90 以上 90 以下。
   * - .. _cfg-launch-longitude:

       ``launch.longitude``
          数値、必須
     - 射点の経度 [deg]（WGS84）。-180 以上 180 以下。
   * - .. _cfg-launch-altitude:

       ``launch.altitude``
          数値、必須
     - 射点の標高 [m]。有限。
   * - .. _cfg-launch-rail_length:

       ``launch.rail_length``
          数値、必須
     - 機体が発射位置からレールに沿って進み、レールを離れるまでの距離 [m]（有効な案内長）。正（:ref:`sec-rail`）。
   * - .. _cfg-launch-elevation_deg:

       ``launch.elevation_deg``
          数値、必須
     - 射角（水平面からの仰角）[deg]。0 より大きく 90 以下（:ref:`sec-initial-attitude`）。
   * - .. _cfg-launch-azimuth_deg:

       ``launch.azimuth_deg``
          数値、必須
     - 射方位 [deg]。北から時計回り。有限（:ref:`sec-initial-attitude`）。
   * - .. _cfg-launch-roll_deg:

       ``launch.roll_deg``
          数値、``0``
     - ランチャ上の機体のロール角 [deg]。機軸まわりに回す。0 では :math:`+y_b` が水平である。有限（:ref:`sec-initial-attitude`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-recovery:

``[recovery]``
--------------

回収系（パラシュート）を指定する。降下の計算は :ref:`page-theory-descent` にある。
``enabled = false`` のときは弾道落下だけを計算する。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-recovery-enabled:

       ``recovery.enabled``
          真偽値、必須
     - パラシュートを使うか。``false`` のとき落下分散は弾道落下だけを計算する。
   * - .. _cfg-recovery-cd_s:

       ``recovery.cd_s``
          数値、必須
     - パラシュートの抗力面積 :math:`C_D S` [m\ :sup:`2`]。二段開傘ではドローグの値。0 以上（``enabled = true`` のときは正）。
   * - .. _cfg-recovery-delay:

       ``recovery.delay``
          数値、必須
     - 頂点から開傘までの遅れ [s]。0 以上。
   * - .. _cfg-recovery-main_cd_s:

       ``recovery.main_cd_s``
          数値、なし
     - 二段開傘のメインの抗力面積 :math:`C_D S` [m\ :sup:`2`]。``main_altitude`` と同時に指定するか、両方とも省略する。メインの開傘高度で抗力面積が ``main_cd_s`` に置き換わる（足し合わせない）。モンテカルロ解析の ``parachute_cd_s_scale`` は ``cd_s`` と ``main_cd_s`` の両方に掛かる。正（:ref:`sec-dual-deploy`）。
   * - .. _cfg-recovery-main_altitude:

       ``recovery.main_altitude``
          数値、なし
     - メインの開傘高度 [m]（射点からの高さ）。下降中にこの高さを横切った時刻に開く。ドローグの開傘時にすでにこの高さ以下なら、ドローグと同時に開く。``main_cd_s`` と同時に指定する。正（:ref:`sec-dual-deploy`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

``main_cd_s`` と ``main_altitude`` は、両方を指定するか両方とも省略する。指定するときは、どちらも有限で正である。

.. _page-reference-config-aero:

.. _sec-config-aero:

.. _cfg-aero:

``[aero]``
----------

空力係数の求め方と、飛翔計算に使う係数表の範囲を指定する。手法の選び方は :ref:`page-theory-aero-choose` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-aero-method:

       ``aero.method``
          文字列、``"barrowman"``
     - 空力係数の求め方。``"barrowman"`` は CAD モデルから抽出した形状に対する部品積み上げ法、``"panel"`` はパネル法、``"cfd"`` は SU2 による CFD、``"table"`` は外部の係数表の読み込みである。
   * - .. _cfg-aero-table:

       ``aero.table``
          パス、なし
     - ``method = "table"`` のとき必須。係数表の CSV ファイル（:ref:`sec-usage-external-table`、:ref:`sec-aero-table-format`）。
   * - .. _cfg-aero-mach_min:

       ``aero.mach_min``
          数値、``0``
     - 係数表の Mach 数の下限。0 以上 ``mach_max`` 未満。
   * - .. _cfg-aero-mach_max:

       ``aero.mach_max``
          数値、``3``
     - 係数表の Mach 数の上限。``mach_min`` より大きく 10 以下。
   * - .. _cfg-aero-mach_step:

       ``aero.mach_step``
          数値、``0.02``
     - 係数表の Mach 数の刻み。格子点の数（Mach 数の数 × 迎角の数）は 10\ :sup:`6` 以下。正。
   * - .. _cfg-aero-alpha_max_deg:

       ``aero.alpha_max_deg``
          数値、``90``
     - 係数表の迎角の上限 [deg]（0 から引く）。表の範囲を超える迎角で引くと、結果は使えないものとして終了ステータス 3 で終わる。正、180 以下（:ref:`sec-exit-status`）。
   * - .. _cfg-aero-alpha_step_deg:

       ``aero.alpha_step_deg``
          数値、``1``
     - 係数表の迎角の刻み [deg]。正。
   * - .. _cfg-aero-n_slices:

       ``aero.n_slices``
          整数、``600``
     - 形状抽出で STL を切る断面の数。20 以上 10000 以下。
   * - .. _cfg-aero-fin_threshold:

       ``aero.fin_threshold``
          数値、``0.05``
     - フィン判定のしきい値（最大胴体半径に対する比）。正。
   * - .. _cfg-aero-extrapolation:

       ``aero.extrapolation``
          文字列、``"linear"``
     - 迎角と横滑り角が表の格子の外のときの扱い。``"linear"`` は線形外挿（符号と範囲を制限する）、``"clamp"`` は端の値で一定にする。Mach 数は、この値にかかわらず端の値で一定である。CFD の表の、解いた迎角より大きい行の埋め方には効かない（:ref:`sec-aero-interp`、:ref:`sec-cfd-table-build`）。
   * - .. _cfg-aero-table_kind:

       ``aero.table_kind``
          文字列、``"axisymmetric"``
     - 飛翔計算に使う係数表の種類。``"axisymmetric"`` は全迎角の軸対称表、``"general"`` は迎角と横滑り角の 6 成分の一般表である（:ref:`sec-aero-table`）。
   * - .. _cfg-aero-general_mach_step:

       ``aero.general_mach_step``
          数値、``0.05``
     - ``table_kind = "general"`` で ``method = "barrowman"`` のときの、一般表の Mach 数の刻み。正（:ref:`sec-aero-table-format`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

``mach_max`` は ``mach_min`` より大きい。
係数表の格子点の数は、軸対称の表（Mach 数 × 迎角）と一般表（Mach 数 × 迎角 × 横滑り角）のうち点の多いほうで判定し、約 10\ :sup:`6` 以下でなければならない。
迎角の数は、0 から ``alpha_max_deg`` までの片側の格子の点の数である。
``table_kind = "general"`` では、外部の表（``method = "table"``）以外のすべての手法が、軸対称の表を一般表へ展開する。

.. _cfg-aero-panel:

``[aero.panel]``
----------------

パネル法の分割と補間の範囲を指定する。モデルは :ref:`page-theory-panel` にある。
``method = "panel"`` のときだけ使うが、値は手法にかかわらず読み込み時に検査し、範囲外なら項目名を示してエラーにする。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-aero-panel-body_axial:

       ``aero.panel.body_axial``
          整数、``80``
     - 胴体のパネルの軸方向の分割数。フィン根元の翼弦の分割を含む。6 以上。
   * - .. _cfg-aero-panel-body_circ:

       ``aero.panel.body_circ``
          整数、``48``
     - 胴体のパネルの周方向の分割数。8 以上。
   * - .. _cfg-aero-panel-fin_chord:

       ``aero.panel.fin_chord``
          整数、``16``
     - フィン片面のパネルの翼弦方向の分割数。2 以上 ``body_axial`` − 4 以下。
   * - .. _cfg-aero-panel-fin_span:

       ``aero.panel.fin_span``
          整数、``10``
     - フィン片面のパネルのスパン方向の分割数。2 以上。
   * - .. _cfg-aero-panel-wake_length:

       ``aero.panel.wake_length``
          数値、``20.0``
     - フィン後流の長さ（全長に対する倍率）。有限で正。
   * - .. _cfg-aero-panel-tail_radii:

       ``aero.panel.tail_radii``
          数値、``6.0``
     - 底面を塞ぐ尾部フェアリング（円筒 + 半球）の円筒部の長さ（底面半径の倍数）。0 は開口である。0 以上。
   * - .. _cfg-aero-panel-subsonic_machs:

       ``aero.panel.subsonic_machs``
          数値の配列、``[0, 0.3, 0.5, 0.6, 0.7, 0.8]``
     - 亜音速のパネル解を計算する Mach 数。この間は pchip で内挿する。1 個以上、各要素は 0 以上 1 未満。
   * - .. _cfg-aero-panel-transonic:

       ``aero.panel.transonic``
          数値の配列（2 要素）、``[0.8, 1.2]``
     - 遷音速の補間区間の Mach 数の範囲 ``[lo, hi]``。``lo`` は純粋に亜音速の列の最後の Mach 数、``hi`` は最初の完全に超音速の Mach 数である。:math:`0<` ``lo`` :math:`<` ``hi``、``hi`` :math:`>1`、``hi`` は ``max(aero.mach_max, 1.2)`` 以下。
   * - .. _cfg-aero-panel-fin_section:

       ``aero.panel.fin_section``
          文字列、``"biconvex"``
     - フィンの断面形状。両凸だけを指定できる。選べる値は ``"biconvex"``。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _page-reference-config-cfd:

.. _cfg-aero-cfd:

``[aero.cfd]``
--------------

``method = "cfd"`` のときだけ使う。SU2、gmsh、Open MPI が必要で、``surface = "stl"`` では manifold3d も要る。
手法と設定の選び方は :ref:`page-theory-cfd-choose` にある。
キーは ``examples/sample_cfd.toml`` の並び（流れと行列、壁面と形状、メッシュ、収束と解法、資源）で載せる。
RANS の既定値は ``model = "rans"`` を選んだときの値である。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-aero-cfd-model:

       ``aero.cfd.model``
          文字列、``"euler"``
     - 支配方程式。``"euler"`` は非粘性の Euler 方程式（粘性の抗力項は部品積み上げ法で補う）、``"rans"`` は Spalart–Allmaras モデルの RANS で、壁面にプリズムの境界層を置いたメッシュで解く。RANS は ``surface = "panel_mesh"``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``"step"`` で使え、``roll_damping`` とは組み合わせられない。``wall_size`` ``iterations`` ``min_iterations`` ``stable_window`` の既定値が変わる（:ref:`sec-cfd-rans-use`）。
   * - .. _cfg-aero-cfd-machs:

       ``aero.cfd.machs``
          数値の配列、``[0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0]``
     - 解く Mach 数。空でない、各要素は正、昇順、重複なし。
   * - .. _cfg-aero-cfd-alphas_deg:

       ``aero.cfd.alphas_deg``
          数値の配列、``[0, 2, 4, 8, 12, 16]``
     - 解く迎角 [deg]。各要素は 0 以上、昇順、重複なし、0 を含み、0 より大きい値が 1 つ以上。
   * - .. _cfg-aero-cfd-alpha_mode:

       ``aero.cfd.alpha_mode``
          文字列、``"mirror"``
     - 迎角の正負と :math:`\alpha=0` のずれの扱い。``"mirror"`` は正負の解の奇関数・偶関数部分を使う（:math:`z` 対称メッシュでは負の迎角は解かない）。``"offset"`` は :math:`\alpha=0` の値を引く。``"single"`` は結果をそのまま使う（:ref:`sec-cfd-alpha-mode`）。
   * - .. _cfg-aero-cfd-betas_deg:

       ``aero.cfd.betas_deg``
          数値の配列、``[0]``
     - 解く横滑り角 [deg]。``[0]`` 以外にすると係数表は 6 成分の一般表になる。空でない、[0, 90] の範囲、昇順、重複なし、0 で始まる（:ref:`sec-cfd-table-kind`）。
   * - .. _cfg-aero-cfd-roll_damping:

       ``aero.cfd.roll_damping``
          真偽値、``false``
     - 各 Mach 数でロール減衰の 2 ケースを回転座標系で解き、:math:`C_{lp}` を CFD の値にする。Euler だけで使える（:ref:`sec-cfd-run`、:ref:`sec-cfd-roll-cases`）。
   * - .. _cfg-aero-cfd-roll_rate_hat:

       ``aero.cfd.roll_rate_hat``
          数値、``0.01``
     - ロール減衰のケースの無次元ロール角速度 :math:`\hat p=pd/(2V)`。0 より大きく 0.05 以下。
   * - .. _cfg-aero-cfd-surface:

       ``aero.cfd.surface``
          文字列、``"panel_mesh"``
     - 壁面の形状。``"panel_mesh"`` は抽出した胴体とフィンを再構成する。``"stl"`` は入力の STL から壁面を作る（入力の形状と同一ではない）。``"step"`` は ``step_file`` の CAD のモデルをそのまま壁面にする（``rocket.stl`` と照合する）。``"stl"`` と ``model = "rans"`` の組合せはエラーである。``"panel_mesh"`` には別名 ``"cad"`` がある（:ref:`sec-cfd-surface`、:ref:`sec-cfd-stl`、:ref:`sec-cfd-fidelity`、:ref:`sec-cfd-step`）。
   * - .. _cfg-aero-cfd-step_file:

       ``aero.cfd.step_file``
          パス、なし
     - ``surface = "step"`` で必須（``rocket.cad`` があるときは省くと、そのファイルを使う）。CAD のファイル。相対パスは設定ファイルのディレクトリを基準にする。``rocket.stl`` と同じ座標系の、一つの閉じた立体として書き出す。``surface = "step"`` 以外ではエラーである。ファイル形式は STEP（``.step`` ``.stp``）、IGES（``.iges`` ``.igs``）、BREP（``.brep`` ``.brp``）（:ref:`sec-cfd-step`）。
   * - .. _cfg-aero-cfd-step_unit:

       ``aero.cfd.step_unit``
          文字列、``"auto"``
     - ``step_file`` の長さの単位（``step_file`` を省いて ``rocket.cad`` を使うときは、``"auto"`` のままなら ``rocket.cad_unit``）。``"auto"`` はファイルが宣言する単位（BREP は m）で読む。単位を宣言していない STEP と IGES では ``"auto"`` を指定できない。``surface = "step"`` 以外ではエラーである。選べる値は ``"auto"``、``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``\ （:ref:`sec-cfd-step`）。
   * - .. _cfg-aero-cfd-base_treatment:

       ``aero.cfd.base_treatment``
          文字列、なし
     - ``surface = "stl"`` または ``"step"`` で Euler のとき必須。``"fairing"`` は底面の後ろに円すいを足す（入力の STL や CAD のモデルを変更する）。``"flat"`` は平らな底面のままにする（Euler の解は収束しないことがあり、収束しないケースは失敗として扱う）。``"fairing"`` は ``tail_fairing`` が正のときだけ指定できる（:ref:`sec-cfd-stl-base`）。
   * - .. _cfg-aero-cfd-stl_remesh:

       ``aero.cfd.stl_remesh``
          文字列、``"classify"``
     - ``surface = "stl"`` の壁面の処理。``"classify"`` は gmsh で曲面を作り ``wall_size`` で作り直す。``"none"`` は STL の三角形をそのまま使う（:ref:`sec-cfd-stl`）。
   * - .. _cfg-aero-cfd-max_geometry_error:

       ``aero.cfd.max_geometry_error``
          数値、``wall_size``
     - ``surface = "stl"`` で、CFD の壁面が入力の STL から離れてよい最大の距離 [m]。超えるとエラー、``wall_size`` の半分を超えると警告する。正（:ref:`sec-cfd-fidelity`）。
   * - .. _cfg-aero-cfd-symmetry:

       ``aero.cfd.symmetry``
          真偽値、``true``
     - ピッチ面（:math:`y=0`）で切った半モデルを使ってよいか。条件を満たさないときと ``false`` は全領域を解く（:ref:`sec-cfd-stl-symmetry`）。
   * - .. _cfg-aero-cfd-shape_symmetry:

       ``aero.cfd.shape_symmetry``
          文字列、``"auto"``
     - 鏡映像として得て解かない対称性。``"axisymmetric"`` は :math:`y=0` と :math:`z=0`、``"pitch_plane"`` は :math:`y=0`、``"none"`` は使わない、``"auto"`` は形状から判定する（:ref:`sec-cfd-stl-symmetry`、:ref:`sec-cfd-table-kind`）。
   * - .. _cfg-aero-cfd-z_mirror_mesh:

       ``aero.cfd.z_mirror_mesh``
          真偽値、``true``
     - 1/4 領域をメッシュにして :math:`z=0` で鏡映し、:math:`z` について厳密に対称なメッシュにするか。形状が :math:`z` について対称でないときと RANS では無視する（:ref:`sec-cfd-stl-symmetry`）。
   * - .. _cfg-aero-cfd-fin_roll_deg:

       ``aero.cfd.fin_roll_deg``
          数値、``0``
     - フィンの組を機軸のまわりに回す角度 [deg]。0 ではフィン 2 枚が :math:`y=0` 面に載り、45 では載らない。有限。
   * - .. _cfg-aero-cfd-general_mach_step:

       ``aero.cfd.general_mach_step``
          数値、``0.1``
     - 一般表の Mach 数の刻み。0 は ``aero.mach_step`` を使う。0 以上（:ref:`sec-aero-table-format`）。
   * - .. _cfg-aero-cfd-tail_fairing:

       ``aero.cfd.tail_fairing``
          数値、``6.0``
     - Euler のみ。平らな底面の後ろに置く円すい形の尾部フェアリングの長さ（底面半径の倍数）。0 は平らな底面。RANS は常に平らな底面である。0 以上（:ref:`sec-cfd-fairing`）。
   * - .. _cfg-aero-cfd-farfield:

       ``aero.cfd.farfield``
          数値、``20.0``
     - 遠方境界の半径（全長の倍数）。半球と球の半径、または箱の半辺。5 以上（:ref:`sec-cfd-domain`）。
   * - .. _cfg-aero-cfd-wall_size:

       ``aero.cfd.wall_size``
          数値、``0.002``\ （RANS は ``0.005``）
     - 壁面の要素の大きさ [m]。RANS では境界層の上面の四面体の大きさでもあり、層の数と厚さが決まる。正（:ref:`sec-cfd-prism`）。
   * - .. _cfg-aero-cfd-fin_size:

       ``aero.cfd.fin_size``
          数値または文字列、``"auto"``
     - フィンの面の要素の大きさ [m]。``"auto"`` は Euler で ``wall_size`` とフィンの厚さの 1/4 の小さいほう、RANS で ``wall_size``。数値を書くと両方のモデルでその大きさにする。値は ``"auto"``、または ``wall_size``/50 以上 ``wall_size`` 以下の長さ [m]（:ref:`sec-cfd-val-asym-cause`）。
   * - .. _cfg-aero-cfd-yplus:

       ``aero.cfd.yplus``
          数値、``1.0``
     - RANS の最初の層の目標 :math:`y^+`\ （全長の 5 % の位置、最大の Mach 数で決める）。その間と 150 を超える値はエラーである。5 以下（壁面まで解像）、または 30 以上 150 以下（標準の壁関数）（:ref:`sec-cfd-prism`）。
   * - .. _cfg-aero-cfd-rans_base:

       ``aero.cfd.rans_base``
          文字列、``"model"``
     - RANS の底面抗力の出どころ。``"model"`` は部品積み上げ法の底面抗力、``"cfd"`` は RANS が平らな底面に積分した力。RANS だけで ``"cfd"`` を指定できる（:ref:`sec-cfd-post`）。
   * - .. _cfg-aero-cfd-iterations:

       ``aero.cfd.iterations``
          整数、``3000``\ （RANS は ``4000``）
     - 1 ケースの最大反復数。RANS では 2 次精度の段の反復数。1 以上。
   * - .. _cfg-aero-cfd-min_iterations:

       ``aero.cfd.min_iterations``
          整数、``100``\ （RANS は ``400``）
     - 早期終了の最小反復数。これより前にはケースを止めない。Euler の Roe の制限関数もこの反復で固定する（RANS の制限関数は固定しない）。``stable_window`` 以上（:ref:`sec-cfd-int-convergence`）。
   * - .. _cfg-aero-cfd-stable_window:

       ``aero.cfd.stable_window``
          整数、``60``\ （RANS は ``200``）
     - 早期終了の判定で、力とモーメントの係数が落ち着いているかを見る直近の反復数。10 以上（:ref:`sec-cfd-int-convergence`、:ref:`sec-cfd-int-rans-accept`）。
   * - .. _cfg-aero-cfd-multigrid:

       ``aero.cfd.multigrid``
          整数、``0``
     - 凝集型マルチグリッドの粗い格子の数（SU2 の ``MGLEVEL``）。0 以上 4 以下（:ref:`sec-cfd-multigrid`）。
   * - .. _cfg-aero-cfd-multigrid_cycle:

       ``aero.cfd.multigrid_cycle``
          文字列、``"w"``
     - マルチグリッドのサイクル。選べる値は ``"w"``、``"v"``\ （:ref:`sec-cfd-multigrid`）。
   * - .. _cfg-aero-cfd-cfl:

       ``aero.cfd.cfl``
          数値、``5.0``
     - CFL 数の初期値（適応的に変える）。正。
   * - .. _cfg-aero-cfd-convergence:

       ``aero.cfd.convergence``
          数値、``1e-6``
     - Euler の密度の二乗平均残差の目標。RANS は冷えた起動の残差の最大値からの下がりで判定する。0 と 1 の間（両端を含まない）（:ref:`sec-cfd-int-rans-accept`）。
   * - .. _cfg-aero-cfd-scheme:

       ``aero.cfd.scheme``
          文字列、``"roe"``
     - 対流フラックスの離散化。選べる値は ``"roe"``、``"jst"``\ （:ref:`sec-cfd-int-discretization`）。
   * - .. _cfg-aero-cfd-prefix:

       ``aero.cfd.prefix``
          文字列、``""``
     - CFD ツールの環境（``<prefix>/bin/SU2_CFD``、``mpirun``、gmsh を持つ ``python``）。空なら環境変数 ``IGNISYEET_CFD_PREFIX``、それも空なら ``PATH`` を使う。先頭の ``~`` は展開する。
   * - .. _cfg-aero-cfd-su2:

       ``aero.cfd.su2``
          文字列、``"SU2_CFD"``
     - SU2 の実行ファイルの名前またはパス。既定のままのときは、環境変数 ``IGNISYEET_SU2_CFD``、``install.sh`` が構築した SU2、``prefix`` と ``PATH`` の ``SU2_CFD`` の順に使う。明示的に書いたパスが優先される。空でない。
   * - .. _cfg-aero-cfd-mpi:

       ``aero.cfd.mpi``
          文字列、``"mpirun"``
     - MPI の起動コマンド。``ranks_per_case`` が 2 以上のときだけ使う。
   * - .. _cfg-aero-cfd-ranks_per_case:

       ``aero.cfd.ranks_per_case``
          整数、``4``
     - 1 ケースの MPI ランク数。0 は 4 とスレッド数の小さいほう。0 以上（0 は自動）（:ref:`sec-cfd-resources`）。
   * - .. _cfg-aero-cfd-parallel_cases:

       ``aero.cfd.parallel_cases``
          整数、``4``
     - 同時に解くケース数。0 はメモリとスレッドの予算に収まる数。``ranks_per_case`` × ``parallel_cases`` が使えるスレッド数を超えると警告する。0 以上（0 は自動）（:ref:`sec-cfd-resources`）。
   * - .. _cfg-aero-cfd-timeout_minutes:

       ``aero.cfd.timeout_minutes``
          数値、``0``
     - 1 ケースの時間の上限 [分]。上限に達したケースは、係数が落ち着いていれば受け入れ、そうでなければ失敗とする。0 以上（0 は無制限）。
   * - .. _cfg-aero-cfd-workdir:

       ``aero.cfd.workdir``
          パス、``"cfd"``
     - ``output.dir`` の下の作業ディレクトリ。ケースごとのサブディレクトリと中間ファイルを置く（:ref:`sec-file-cfd`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _page-reference-config-environment:

.. _cfg-earth:

``[earth]``
-----------

積分する座標系と重力モデルを指定する。モデルの式と選び方は :ref:`page-theory-atmosphere-gravity` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-earth-model:

       ``earth.model``
          文字列、``"flat"``
     - 積分する座標系。``"flat"`` は射点の局所 ENU 座標系（平面地球・慣性系）、``"ecef"`` は WGS84 楕円体に固定した回転座標系（Coriolis 力と遠心力を含む）（:ref:`sec-ecef`）。
   * - .. _cfg-earth-gravity:

       ``earth.gravity``
          文字列、``"inverse_square"``
     - 重力モデル。``"constant"`` は :math:`g_0` 一定、``"inverse_square"`` は逆 2 乗則、``"j2"`` は点質量に :math:`J_2` 項を加える。``"j2"`` は ``earth.model = "ecef"`` のときだけ指定できる（:ref:`sec-gravity`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-atmosphere:

``[atmosphere]``
----------------

大気のモデルを指定する。この設定は飛翔計算と、摩擦抗力の Reynolds 数（射点高度の大気）に使われる。
モデルの式と選び方は :ref:`page-theory-atmosphere-gravity` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 30 70

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-atmosphere-model:

       ``atmosphere.model``
          文字列、``"us1976"``
     - 大気モデル。``"us1976"`` は米国標準大気 1976、``"constant"`` は高度によらず一定の大気。
   * - .. _cfg-atmosphere-temperature_offset:

       ``atmosphere.temperature_offset``
          数値、``0.0``
     - 標準大気の温度に一律に足す温度差 [K]。``us1976`` のみ。有限で、-186.946 より大きい。
   * - .. _cfg-atmosphere-density:

       ``atmosphere.density``
          数値、``1.225``
     - 密度 [kg/m\ :sup:`3`]。``constant`` のみ。有限で正。
   * - .. _cfg-atmosphere-sound_speed:

       ``atmosphere.sound_speed``
          数値、``340.29``
     - 音速 [m/s]。``constant`` のみ。有限で正。
   * - .. _cfg-atmosphere-viscosity:

       ``atmosphere.viscosity``
          数値、``1.789e-5``
     - 粘性係数 [Pa s]。``constant`` のみ。有限で正。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-wind:

``[wind]``
----------

風を指定する。``profile`` 以外のモデルは、基準高度 ``ref_height`` での風速 ``speed`` と風向 ``direction_deg`` から、高度分布の式で上空の風を決める。
``profile`` は高度別の表から風を決めるので、``speed`` と ``direction_deg`` は使わない（書いてあると警告する）。
モデルの式と選び方は :ref:`page-theory-wind` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 30 70

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-wind-model:

       ``wind.model``
          文字列、``"power"``
     - 風速の高度分布。``"constant"`` は一定風、``"power"`` はべき法則、``"log"`` は対数則、``"profile"`` は高度別の表。
   * - .. _cfg-wind-speed:

       ``wind.speed``
          数値、``4.0``
     - 基準高度での風速 [m/s]。0 以上。
   * - .. _cfg-wind-direction_deg:

       ``wind.direction_deg``
          数値、``0.0``
     - 風が吹いてくる方位 [deg]（北から時計回り）。0 以上 360 以下。
   * - .. _cfg-wind-ref_height:

       ``wind.ref_height``
          数値、``2.0``
     - 風速の基準高度 [m]。正。
   * - .. _cfg-wind-exponent:

       ``wind.exponent``
          数値、``6.0``
     - べき法則の指数 :math:`n`。``power`` のみ。正。
   * - .. _cfg-wind-roughness_length:

       ``wind.roughness_length``
          数値、``0.03``
     - 粗度長 :math:`z_0` [m]。``log`` のみ。0 より大きく、``ref_height`` の半分より小さい。
   * - .. _cfg-wind-ground_blend_height:

       ``wind.ground_blend_height``
          数値、``1.0``
     - 地面付近で風速を 0 になめらかにつなぐ高さ [m]。``power`` と ``log`` のみ。正で、``ref_height`` より小さい。``log`` では :math:`z_0` より大きい必要があり、:math:`z_0` 以下なら :math:`\max(\text{値}, 2 z_0)` に読み替える。
   * - .. _cfg-wind-profile:

       ``wind.profile``
          パス、なし
     - 高度別の風の CSV ファイル。``model = "profile"`` のとき必須。
   * - .. _cfg-wind-profile_mean_speed:

       ``wind.profile_mean_speed``
          数値、なし
     - 表の平均風速 :math:`W` をこの値 [m/s] にするよう、表のすべての風速を同じ倍率で拡大・縮小する。``profile`` のみ。有限で 0 以上。省略すると表のまま。
   * - .. _cfg-wind-profile_mean_direction_deg:

       ``wind.profile_mean_direction_deg``
          数値、なし
     - 表の平均風の風向 :math:`\Psi` をこの方位 [deg] にするよう、表のすべての風向を同じ角度だけ回す。``profile`` のみ。0 以上 360 以下。省略すると表のまま。

.. raw:: latex

   \renewcommand{\arraystretch}{1}
.. _page-reference-config-sim:

.. _sec-config-sim:

.. _cfg-sim:

``[sim]``
---------

数値積分と飛翔計算の出力を指定する。積分法の違いは :ref:`page-theory-integrators` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-sim-integrator:

       ``sim.integrator``
          文字列、``"rk4"``
     - 数値積分法。``"rk4"`` は固定刻みの古典的 Runge–Kutta 法、``"rk45"`` は刻み幅制御付きの Dormand–Prince 5(4) 法、``"dop853"`` は刻み幅制御付きの Dormand–Prince 8(5,3) 法（:ref:`sec-integrators`）。
   * - .. _cfg-sim-attitude:

       ``sim.attitude``
          文字列、``"normalize"``
     - 姿勢の更新法。``"normalize"`` は四元数を通常の状態量として積分し、各ステップ後に正規化する。``"lie_group"`` は SO(3) 上の Runge–Kutta–Munthe-Kaas 法で、正規化をせずに四元数のノルムが丸め誤差の範囲で 1 に保たれる。どの積分法とも組み合わせられる（:ref:`sec-integrators`）。
   * - .. _cfg-sim-dt:

       ``sim.dt``
          数値、``0.002``
     - ``rk4`` の時間刻み [s]。``rk45`` と ``dop853`` では最初の刻み幅として使う。``rk4`` では ``max_time / dt`` が 1e8 以下。1e-5 以上 0.05 以下。
   * - .. _cfg-sim-rtol:

       ``sim.rtol``
          数値、``1e-7``
     - ``rk45`` と ``dop853`` の相対許容誤差。正で 1e-3 以下。
   * - .. _cfg-sim-atol:

       ``sim.atol``
          数値、``1e-6``
     - ``rk45`` と ``dop853`` の絶対許容誤差。正で 1e-3 以下。
   * - .. _cfg-sim-max_time:

       ``sim.max_time``
          数値、``3600``
     - 計算を打ち切る時刻 [s]。この時刻までに着地しなかった飛翔は着地に失敗したものとして扱い、``sim`` は警告して終了ステータス 3 で終わる。落下分散では、そのケースとサンプルを失敗に数える。正（:ref:`sec-exit-status`）。
   * - .. _cfg-sim-output_interval:

       ``sim.output_interval``
          数値、``0.05``
     - ``trajectory.csv`` に書き出す間隔 [s]。``rk45`` と ``dop853`` では刻み幅の上限でもある（燃焼中とレール上は 0.05 s 以下）。1e-4 以上 ``max_time`` 以下。``max_time / output_interval`` は 1e7 以下（:ref:`sec-file-trajectory`）。
   * - .. _cfg-sim-descent:

       ``sim.descent``
          文字列、自動
     - ``sim`` の降下モード。省略時は ``recovery.enabled`` が ``true`` なら ``"parachute"``、``false`` なら ``"ballistic"``。``"parachute"`` は ``recovery.enabled = true`` のときだけ指定できる。
   * - .. _cfg-sim-quasi_steady:

       ``sim.quasi_steady``
          真偽値、``false``
     - ``true`` にすると、加速度と回転による空気力のうち、見かけの質量の遅れと対流の回転の項、Magnus 力とモーメント、回転による法線力を切って飛ばす。静的な係数、減衰のモーメント、ロールの項、誘導ロールは残る。設定の一覧に ``aerodynamics: quasi-steady only`` の行が出る（:ref:`sec-aero-unsteady`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

``integrator = "rk4"`` では ``max_time / dt`` が 10\ :sup:`8` 以下でなければならない。
``rk45`` と ``dop853`` は ``dt`` で刻まないので、この検査はしない。
``output_interval`` は ``max_time`` 以下で、``max_time / output_interval`` は 10\ :sup:`7` 以下である。
``descent = "parachute"`` は ``recovery.enabled = true`` のときだけ指定できる。コマンドラインの ``sim --descent parachute`` も同じである。

.. _page-reference-config-dispersion:

.. _cfg-dispersion:

``[dispersion]``
----------------

落下分散の計算方式を指定する。風の格子とモンテカルロ解析の考え方は :ref:`page-theory-dispersion` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-dispersion-mode:

       ``dispersion.mode``
          文字列、``"wind_grid"``
     - ``"wind_grid"`` は風速 × 風向の格子、``"monte_carlo"`` はモンテカルロ解析。
   * - .. _cfg-dispersion-wind_speeds:

       ``dispersion.wind_speeds``
          数値の配列、``[1, 2, 3, 4, 5, 6, 7]``
     - ``wind_grid`` で計算する風速の一覧 [m/s]。``constant`` ``power`` ``log`` では基準高度 ``wind.ref_height`` の風速、``profile`` では表の平均風速。1 個以上 100 個以下、各要素は 0 以上（:ref:`sec-wind-scaling`、:ref:`sec-dispersion-safety`）。
   * - .. _cfg-dispersion-directions:

       ``dispersion.directions``
          整数、``8``
     - ``wind_grid`` の風向の分割数。北から等間隔に分ける。風の格子のケース数（``wind_speeds`` の数 × ``directions``）は 100000 以下。1 以上 360 以下。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-dispersion-monte_carlo:

``[dispersion.monte_carlo]``
----------------------------

``dispersion.mode = "monte_carlo"`` のときに使う。
各ばらつきは互いに独立な正規分布 :math:`N(0,\sigma^2)` から引き、このキーの値は標準偏差 :math:`\sigma` である。
0 はそのばらつきを無効にする。負の値や有限でない値はエラーである。
「相対」は公称値に対する割合（0.03 は 3 %。公称値に :math:`1+\text{乱数}` を掛け、下限は 0.05）、「絶対」は公称値に加える量である。
ばらつきのキーは乱数を引く順に並べてある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :class: longtable
   :header-rows: 1
   :widths: 40 60

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-dispersion-monte_carlo-samples:

       ``dispersion.monte_carlo.samples``
          整数、``1000``
     - サンプル数。1 以上 1e6 以下（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-seed:

       ``dispersion.monte_carlo.seed``
          整数、``1``
     - 乱数の種。サンプル :math:`i` は ``seed + i`` を種とする。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-thrust_scale:

       ``dispersion.monte_carlo.thrust_scale``
          数値、``0.03``
     - 相対。推力（全力積が比例して変わる。推進剤質量は変えない）。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-burn_time_scale:

       ``dispersion.monte_carlo.burn_time_scale``
          数値、``0.02``
     - 相対。燃焼時間の伸縮（推力曲線の時間軸を伸ばす）。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-dry_mass:

       ``dispersion.monte_carlo.dry_mass``
          数値、``0.1``
     - 相対。乾燥質量。乾燥慣性テンソルのすべての要素に同じ倍率を掛ける。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-cg:

       ``dispersion.monte_carlo.cg``
          数値、``0.01``
     - 絶対 [m]。乾燥重心の位置（正が後方）。慣性モーメントは変えない。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-cn_scale:

       ``dispersion.monte_carlo.cn_scale``
          数値、``0.10``
     - 相対。法線力係数 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-ca_scale:

       ``dispersion.monte_carlo.ca_scale``
          数値、``0.15``
     - 相対。軸力（抗力）係数 :math:`C_A`\ （燃焼中・燃焼後）。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-elevation_deg:

       ``dispersion.monte_carlo.elevation_deg``
          数値、``0.5``
     - 絶対 [deg]。レールの向きの設置誤差 :math:`\sigma_{\mathrm{tilt}}`。公称の軸を、面内（射角）と面に垂直な 2 方向の独立な傾きで等方に傾ける。機体はレールとともに回る。傾けた射角が :math:`(0,90]` の外のサンプルは失敗になる。0 以上（:ref:`sec-mc-rail`）。
   * - .. _cfg-dispersion-monte_carlo-azimuth_deg:

       ``dispersion.monte_carlo.azimuth_deg``
          数値、``1.0``
     - 絶対 [deg]。射方位の設定誤差。傾けたレールを機体ごと鉛直軸の周りに回す（水平方向の効果は :math:`\sigma\cos(\text{射角})`）。0 以上（:ref:`sec-mc-rail`）。
   * - .. _cfg-dispersion-monte_carlo-wind_speed:

       ``dispersion.monte_carlo.wind_speed``
          数値、``1.0``
     - 絶対 [m/s]。基準風速（``wind.speed`` の周りに分布し、0 未満は 0 にする）。``profile`` では表の平均風速 :math:`W`\ （``profile_mean_speed`` を指定したときはその値）の周りの揺らぎで、表を :math:`\max(W+\delta,0)/W` 倍する。表がほぼ無風（平均 0.1 m/s 未満）でこの値が 0 でないときはエラーである。0 以上（:ref:`sec-wind-scaling`）。
   * - .. _cfg-dispersion-monte_carlo-wind_direction_deg:

       ``dispersion.monte_carlo.wind_direction_deg``
          数値、``15.0``
     - 絶対 [deg]。風向（``wind.direction_deg`` の周りに分布する。``profile`` では表の平均風の風向の周り。``profile_mean_direction_deg`` を指定したときはその方位の周り）。0 以上（:ref:`sec-wind-scaling`）。
   * - .. _cfg-dispersion-monte_carlo-parachute_cd_s_scale:

       ``dispersion.monte_carlo.parachute_cd_s_scale``
          数値、``0.1``
     - 相対。パラシュートの抗力面積 :math:`C_DS`。``recovery.main_cd_s`` にも同じ倍率を掛ける。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-thrust_misalignment_deg:

       ``dispersion.monte_carlo.thrust_misalignment_deg``
          数値、``0``
     - 絶対 [deg]。推力の傾き。:math:`y_b`、:math:`z_b` 方向の傾き成分に独立な正規乱数を足し、傾きの角度と方位を決める。0 以上（:ref:`sec-thrust-line`）。
   * - .. _cfg-dispersion-monte_carlo-thrust_offset:

       ``dispersion.monte_carlo.thrust_offset``
          数値、``0``
     - 絶対 [m]。ノズル出口の点。:math:`y_b`、:math:`z_b` 方向に独立な正規乱数を足す。0 以上（:ref:`sec-thrust-line`）。
   * - .. _cfg-dispersion-monte_carlo-fin_cant_deg:

       ``dispersion.monte_carlo.fin_cant_deg``
          数値、``0``
     - 絶対 [deg]。設定したフィンのカントに加える量。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-cg_lateral:

       ``dispersion.monte_carlo.cg_lateral``
          数値、``0``
     - 絶対 [m]。乾燥重心の横ずれ。:math:`y_b`、:math:`z_b` 方向に独立な正規乱数を足す。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-cl_scale:

       ``dispersion.monte_carlo.cl_scale``
          数値、``0``
     - 相対。ロールのモーメント係数 :math:`C_l` とカントの傾き :math:`C_{l\delta}`。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-clp_scale:

       ``dispersion.monte_carlo.clp_scale``
          数値、``0``
     - 相対。ロール減衰微係数 :math:`C_{lp}`。0 以上（:ref:`sec-mc`）。
   * - .. _cfg-dispersion-monte_carlo-lag_scale:

       ``dispersion.monte_carlo.lag_scale``
          数値、``0``
     - 相対。遅れの和 ``lag_l0``、``lag_l1``\ （胴体とフィンの見かけの質量の遅れと対流の回転の項）。0 以上（:ref:`sec-aero-unsteady`、:ref:`sec-quasi-steady`）。
   * - .. _cfg-dispersion-monte_carlo-magnus_scale:

       ``dispersion.monte_carlo.magnus_scale``
          数値、``0``
     - 相対。Magnus 力とモーメントの傾き ``cnpa``、``cmpa``。組み込みの手法はこれらを推算しないので、外部の表で値を与えたときだけ効く（値がなければ警告する）。0 以上（:ref:`sec-aero-unsteady`）。
   * - .. _cfg-dispersion-monte_carlo-cmq_scale:

       ``dispersion.monte_carlo.cmq_scale``
          数値、``0``
     - 相対。ピッチ・ヨーの減衰の和（減衰のモーメントと回転による法線力）。``cn_scale`` の倍率に重ねて掛ける。見かけの質量の項には掛からない。0 以上（:ref:`sec-quasi-steady`）。
   * - .. _cfg-dispersion-monte_carlo-truncate_sigma:

       ``dispersion.monte_carlo.truncate_sigma``
          数値、なし
     - 指定すると、引いたすべての標準正規乱数 :math:`z` を :math:`[-k,k]`\ （:math:`k` はこの値）に切ってから標準偏差を掛ける。省略すると切らない正規分布である。有限で正（:ref:`sec-mc`）。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

``thrust_misalignment_deg`` から ``cmq_scale`` までの 9 項目は既定で無効である。
この 9 項目のどれを有効にしても、最初の 11 項目から引く値は変わらない。また ``lag_scale`` ``magnus_scale`` ``cmq_scale`` の設定は、ほかのどの項目の値も変えない。

.. _sec-config-checks:

値の検査
--------

各キーの型と範囲は、上の表の意味の欄に示した。複数のキーにまたがる条件は、それぞれの表の後ろに書いた。
ここでは、表のキーに結び付かない検査をまとめる。検査にかかったときのメッセージと対処は :ref:`errors` にある。

``wind.model`` が使わないキーは検査せず、使わないモデルではどの値でもエラーにしない。
``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` は ``model = "profile"`` のときだけ指定できる（:ref:`IY-C056`）。
どのセクションも、未知のキーがあるとキーの名前を示してエラーにする。綴りの誤りが黙って無視されることはない。

入力ファイル（STL、モータの ``.eng``、風のプロファイル、外部の係数表とその付随ファイル）が読めないときは、ファイルのパスを示してエラーにする。
STL は、頂点の座標に有限でない値があるとき、バイナリ STL が見出しの三角形の数に対して短いとき、表面が閉じていないときにもエラーにする（:ref:`sec-geometry-warnings`）。
``.eng`` の検査は :ref:`sec-propulsion-eng` に、外部の係数表の検査は :ref:`sec-aero-table-format` にある。

形状を抽出したあとで、次の場合にエラーにする。

* 機体の全長が許容範囲の外にある。``stl_scale`` の誤りを疑う（範囲は :ref:`page-theory-geometry` にある）。
* 乾燥重心 ``rocket.cg_dry`` が機体の全長より後方にある。
* モータ（``.eng`` の長さ）が機体の全長より長い。
* モータ後端の位置 ``motor.aft_x`` がモータの長さより前、または機体の全長より後方にあり、モータが機体の外に出る。どちらも 1 mm の許容がある。

全長と乾燥重心の検査のエラーでは、形状の抽出の警告（:ref:`sec-geometry-warnings`）をメッセージの後に続けて示す。

積分が発散して状態に有限でない値が現れたときも、速さや高さが現実にあり得ない値になったときも、エラーにする（:ref:`sec-integrators`）。
モンテカルロ解析ではそのサンプルを失敗として数え、風の格子ではそのケースだけを失敗として記録して、ほかのケースの計算は続ける（:ref:`sec-wind-grid-failed`）。

飛翔計算の前に、乾燥時と燃焼中の慣性テンソルを検査し、主慣性モーメントが正でない、または極端に偏るときはエラーにする（:ref:`sec-inertia-tensor`）。
回収系が無効な機体をパラシュート降下で飛ばそうとするとエラーにする。回収系が有効なときは、``recovery.cd_s`` が有限で正であること、``recovery.delay`` が有限で 0 以上であることも飛翔計算の前に確かめる。

UTF-8 の BOM（バイト順マーク）が先頭にあっても読める（設定ファイル、``.eng``、外部の係数表とその JSON、風のプロファイルの表）。
