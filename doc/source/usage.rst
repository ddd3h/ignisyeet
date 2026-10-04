使い方
======

必要なもの
----------

.. list-table::
   :header-rows: 1
   :widths: 25 75

   * - ソフトウェア
     - 用途
   * - Rust（stable）と ``cargo``
     - 本体（形状抽出・空力解析・飛翔計算・落下分散）のビルド
   * - `uv <https://docs.astral.sh/uv/>`_ と Python 3.10 以上（uv の代わりに ``venv`` と ``pip`` でもよい）
     - 作図（numpy, pandas, matplotlib）。uv なら自動で導入する。pip の場合は ``python/requirements.txt`` を使う（:ref:`最初の実行の節 <sec-python-env>`）
   * - LuaLaTeX（luatexja）と dvisvgm
     - このドキュメントの TikZ 図を作るときのみ
   * - upLaTeX、dvipdfmx、latexmk と inkscape
     - このドキュメントの PDF 版を作るときのみ（:ref:`sec-doc-build`）

Rust と Python について
~~~~~~~~~~~~~~~~~~~~~~~

IgnisYeet は、計算の重い部分を Rust、作図を Python で書いている。二つの言語の特徴と、この役割分担にした理由を簡単に述べる。

Rust
   コンパイルして機械語の実行ファイルを作る言語で、C や C++ と同程度の速さで動く。
   特徴は、メモリの扱いの誤り（解放済みメモリの参照や、複数のスレッドからの同時書き込みなど）を、
   **実行前のコンパイルの段階で検出する** ことである。ガベージコレクタを使わずにこれを実現するので、速さを損なわない。
   このため、数十万ステップの数値積分や、パネル法の大きな連立一次方程式、
   数千ケースの落下分散を複数の CPU コアで並列に計算する部分を、安全かつ高速に書ける。
   ビルド・テスト・依存ライブラリの管理は付属の ``cargo`` がまとめて行う。
   インストールは公式の ``rustup``\ （https://rustup.rs）を使うのが簡単である。

Python
   コンパイル不要で、そのまま実行できる言語である。文法が簡潔で読みやすく、
   `numpy <https://numpy.org/>`__\ （数値配列）、`pandas <https://pandas.pydata.org/>`__\ （表データ）、`matplotlib <https://matplotlib.org/>`__\ （グラフ）といった科学技術計算向けのライブラリが充実している。
   一方、Python で書いたループは Rust に比べて桁違いに遅い。
   そこで IgnisYeet では、計算結果の CSV を読み込んで図にする部分だけを Python で書いている。
   ライブラリの導入には、標準の ``pip``\ （仮想環境 ``venv`` と組み合わせる）か、
   より高速なパッケージ管理ツール ``uv`` を使う。

計算部分と作図部分は CSV・JSON ファイルを介してつながっているだけなので、
作図を別のツール（Excel、gnuplot、MATLAB など）で行うこともできる。

インストール
------------

.. code-block:: sh

   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   cargo build --release      # target/release/ignisyeet ができる

最初の実行
----------

付属のサンプル機体（直径 100 mm、全長 1.5 m、4 枚フィン）で全工程を実行する。

.. code-block:: sh

   make all

これは次のコマンドを順に実行するのと同じである。

.. code-block:: sh

   ./target/release/ignisyeet aero       examples/sample.toml   # 空力係数表を作る（初回のみ計算）
   ./target/release/ignisyeet sim        examples/sample.toml   # 1 回の飛翔を計算する
   ./target/release/ignisyeet dispersion examples/sample.toml   # 落下分散を計算する
   uv run --project python python/plot.py all examples/out      # 図を描く

``python/plot.py`` は ``geometry`` ``aero`` ``trajectory`` ``dispersion`` のいずれかを選んで描くこともできる。
``dispersion`` は ``dispersion.csv`` があれば風の格子の図（``dispersion.png``）を、
``dispersion_mc.csv`` と ``dispersion_summary.json`` があればモンテカルロの図（``dispersion_mc.png``）を描く（両方あれば両方描く）。
``aero`` に ``--compare <別の出力ディレクトリ>`` を付けると、二つの係数表の :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{D0}` を Mach 数に対して重ねた図
（``aero_compare.png``。凡例はディレクトリ名）も描く。

.. _sec-python-env:

uv を使わずに ``pip`` で作図する場合は、仮想環境を作って依存パッケージを入れ、``python`` から直接実行する。

.. code-block:: sh

   python3 -m venv .venv && . .venv/bin/activate
   pip install -r python/requirements.txt
   python python/plot.py all examples/out
   # make 経由なら: make plot PLOT="python python/plot.py"

自分の機体で計算するときは、``examples/sample.toml`` をコピーして編集し、
``make all CONFIG=path/to/rocket.toml`` とすればよい。

端末の表示
----------

``ignisyeet`` を端末で実行すると、設定の一覧、進捗バー、結果の要約を色付きの枠で表示する。
表示は標準出力が端末かどうかで切り替わり、パイプやファイルへの出力では装飾のない行単位の出力になる（:ref:`sec-ui-modes`）。

画面の構成
~~~~~~~~~~

起動すると、まずバナー（バージョンと概要）と設定パネル（Configuration）を出す。
設定パネルには、入力ファイル、機体、射点、回収、空力、環境（地球・大気・風）、積分、出力先が項目ごとにまとまって並ぶ。
設定ファイルの内容を実行前に確認できるので、単位や風向の入れ間違いに気づきやすい。

.. figure:: _generated/screens/screen_sim_start.png
   :width: 100%
   :alt: sim の起動画面（バナーと設定パネル）

   ``sim`` の起動画面。水色の見出しで項目が分かれ、設定値は太字で示す。方式名（``barrowman``）は紫で表示する。

その後は工程ごとに、実行中は進捗バーを、終了すると完了行（チェック印と処理名、要約、所要時間）を表示する。
進捗バーは実行中だけ表示し、工程が終わると完了行に置き換わる。

``aero``
   Barrowman 法では係数表の行数を分母にしたバーを出す。パネル法では段階（形状の準備、亜音速、超音速など）ごとに進み具合を示し、
   2 行目に実行中の処理を表示する。経過時間と残り時間の見積り（ETA。黄色）を併記する。保存済みの表を使うときは、バーを出さずに ``cached`` と行数を表示する。

.. figure:: _generated/screens/screen_aero_panel.png
   :width: 100%
   :alt: aero のパネル法の実行中の画面

   ``aero``\ （``method = "panel"``）の実行中。形状の要約に続いて、段階の進み具合（4/9）と ETA、実行中の処理を表示する。

``sim``
   飛翔の段階（``rail`` ``powered`` ``coast`` ``parachute`` など）と、その時点の高度・速度・Mach 数・静安定余裕を 2 行目に更新し続ける。
   バーの分母は目標とする飛行時間で、飛翔の途中で見積りを更新するため、ETA と終点（``~684s`` のような表示）は計算が進むにつれて補正される。

``dispersion``
   終わったケース数のバーに、1 秒あたりの処理数、経過時間、ETA を表示する。2 行目には、ここまでの最大着地距離（弾道落下とパラシュート降下）を示す。
   ``monte_carlo`` ではサンプル数が分母になる。

.. figure:: _generated/screens/screen_dispersion.png
   :width: 100%
   :alt: dispersion（モンテカルロ）の実行中の画面

   ``dispersion``\ （``mode = "monte_carlo"``）の実行中。処理数、1 秒あたりの処理数、ETA と、ここまでの最大着地距離を表示する。

完了すると結果パネルを出す。``sim`` では Flight result パネルにランチャ離脱、最大速度・最大動圧、頂点、着地点などをまとめる。
最後に、書き出したファイルの一覧（パスとサイズ）と、全体の所要時間を表示する。

.. figure:: _generated/screens/screen_sim_result.png
   :width: 100%
   :alt: sim の完了画面（結果パネルと出力ファイル）

   ``sim`` の完了画面。結果パネル、書き出したファイルの一覧、所要時間が並ぶ。

結果パネルの静安定余裕には判定の印が付く。

.. list-table::
   :header-rows: 1
   :widths: 28 32 40

   * - 静安定余裕
     - 表示
     - 意味
   * - 1.5 cal 以上
     - 緑のチェック印
     - 十分な余裕がある
   * - 1.0 cal 以上 1.5 cal 未満
     - 黄の警告記号と ``marginal (< 1.5 cal)``
     - 余裕が小さい。突風や重心のずれで不安定に近づく
   * - 1.0 cal 未満
     - 赤のバツ印と ``UNSTABLE``
     - 不安定。設計を見直す

.. _sec-ui-modes:

出力モードと環境変数
~~~~~~~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 条件
     - 動作
   * - 標準出力が端末
     - 上で述べた表示（バナー、パネル、色）。進捗バーは標準エラー出力が端末のときだけ出す。
   * - 標準出力をパイプ・リダイレクトした
     - プレーンモード。バナー、パネル、色、進捗バーを出さず、行単位の出力（``Flight (...)`` や ``rail exit ...`` など）を書く。
       ``ignisyeet sim ... | tee log.txt`` のように記録しても制御文字が混ざらない。
   * - ``-q`` / ``--quiet``
     - エラーと、書き出したファイルのパスだけを表示する。スクリプトから呼ぶときに使う。
   * - ``--no-progress``
     - 進捗バーとスピナーを出さない。枠付きの要約は表示する。ログに残したい場合などに使う。
   * - ``NO_COLOR`` が空でない値で設定されている
     - 色を使わない（枠や記号は残る）。
   * - ``TERM=dumb``
     - 色と Unicode の記号を使わない。
   * - ロケールが UTF-8 でない（``LANG`` ``LC_ALL`` ``LC_CTYPE`` が UTF-8 を指さない、または ``TERM=linux``）
     - 枠線・記号・バーを ASCII 文字（``+ - |``、``ok`` ``x`` ``!``、``#>-``）に置き換える。

.. code-block:: sh

   ./target/release/ignisyeet sim examples/sample.toml -q          # 書き出したパスだけ
   ./target/release/ignisyeet dispersion examples/sample.toml --no-progress
   NO_COLOR=1 ./target/release/ignisyeet aero examples/sample.toml
   ./target/release/ignisyeet sim examples/sample.toml | cat        # プレーンモード

サブコマンド
------------

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - コマンド
     - 内容
   * - ``geom <config>``
     - STL から形状を抽出し、``geometry.json`` と ``profile.csv`` を書き出す。抽出結果の確認用。
   * - ``aero <config> [--force]``
     - 空力係数表 ``aero_table.csv`` を作る。入力（STL と空力関係の設定）が前回と同じなら保存済みの表を使う。
       ``--force`` を付けると必ず作り直す。
   * - ``sim <config> [--descent ballistic|parachute]``
     - 設定した風で 1 回飛翔させ、``trajectory.csv`` と ``summary.json`` を書き出す。
   * - ``dispersion <config>``
     - ``dispersion.mode`` に従って落下分散を並列に計算する。``"wind_grid"`` では風速 × 風向の全ケースを計算して ``dispersion.csv`` を、
       ``"monte_carlo"`` では各サンプルの結果 ``dispersion_mc.csv`` と統計 ``dispersion_summary.json`` を書き出す。
   * - ``sample-stl <path>``
     - 付属のサンプル機体の STL（mm 単位、ノーズ +z 向き）を書き出す。

``sim`` と ``dispersion`` は、係数表がない場合や入力が変わった場合に自動で ``aero`` を実行する。
入力の同一性は、STL の内容と設定値から計算したハッシュ値（FNV-1a）で判定する。
プログラム本体を更新したときは、係数の計算方法が変わっている可能性があるので ``aero --force`` を実行してほしい。

設定ファイル
------------

設定は `TOML <https://toml.io/ja/v1.0.0>`_ 形式で書く。
相対パスは **設定ファイルが置かれたディレクトリ** を基準に解釈する。
長さの単位は m で、機体上の位置はすべて **ノーズ先端から後方へ測る**。
各項目の既定値と意味を以下に示す（既定値が「必須」のものは省略できない）。

``[output]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``dir``
     - ``"out"``
     - 出力先ディレクトリ
   * - ``kml``
     - ``true``
     - Google Earth 用の KML ファイル ``ignisyeet.kml`` も書き出す（:ref:`sec-kml`）

``[resources]``\ （計算資源。すべて省略できる。結果は変わらないので、空力係数表のハッシュにも含めない）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``threads``
     - ``0``
     - 全処理で使う CPU スレッド数の上限。``0`` は使用可能な全スレッド。使用可能数を超える値は警告して使用可能数に切り詰める。分散計算、モンテカルロ、パネル法の行列計算、CFD の MPI ランク数に効く
   * - ``memory_gb``
     - ``0``
     - 重い処理のメモリ予算 [GB]。``0`` は制限なし。パネル法は密行列に必要な量（パネル数 n に対しておよそ 16 n² バイト）が予算を超えると、計算を始める前にエラーで止まる。CFD は予算に収まるよう同時実行ケース数を減らす。負の値はエラー
   * - ``nice``
     - ``0``
     - プロセスの優先度を下げる値（0 から 19）。長時間の計算で他の作業を妨げたくないときに使う。SU2 や ``mpirun`` の子プロセスにも引き継がれる。Windows など Unix 以外では警告して無視する。範囲外の値はエラー

実際に使うスレッド数、メモリ予算、``nice`` は、起動時の設定パネルに表示する（プレーン表示では 1 行で示す）。

``[rocket]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``stl``
     - 必須
     - 機体の STL ファイル（ASCII / バイナリのどちらでもよい）
   * - ``stl_scale``
     - ``1.0``
     - STL の単位を m に直す係数。mm で作った STL なら ``0.001``
   * - ``nose_direction``
     - ``"auto"``
     - STL 座標でノーズが向いている方向。``"+x"`` ``"-x"`` ``"+y"`` ``"-y"`` ``"+z"`` ``"-z"`` で指定できる
   * - ``dry_mass``
     - 必須
     - 推進剤を除いた質量 [kg]（空のモータケースを含む）
   * - ``cg_dry``
     - 必須
     - 推進剤を除いたときの重心位置 [m]
   * - ``ixx_dry`` / ``iyy_dry``
     - 必須
     - 推進剤を除いたときのロール / ピッチ・ヨー慣性モーメント [kg m\ :sup:`2`]（``iyy_dry`` は乾燥重心まわり）
   * - ``roughness``
     - ``60e-6``
     - 表面の等価砂粒粗さ [m]。一般的な塗装面で 60 µm 程度
   * - ``fin_le``
     - ``"rounded"``
     - フィン前縁形状: ``"rounded"`` （丸め）または ``"sharp"`` （鋭角）
   * - ``fin_te``
     - ``"square"``
     - フィン後縁形状: ``"square"`` （切り落とし、底面抗力あり）または ``"tapered"``
   * - ``extra_cd``
     - ``0.0``
     - ランチャラグなどの付加的な抗力係数
   * - ``[rocket.fin_override]``
     - なし
     - フィンの自動検出結果を上書きする。``count`` ``root_chord`` ``tip_chord`` ``span`` ``sweep`` ``thickness`` ``x_le_root`` を必要なものだけ書く

``[motor]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``eng``
     - 必須
     - RASP 形式（``.eng``）の推力曲線ファイル
   * - ``aft_x``
     - 機体全長
     - モータ後端の位置 [m]
   * - ``nozzle_exit_diameter``
     - ``0.0``
     - ノズル出口直径 [m]。燃焼中はこの面積だけ底面抗力が減る
   * - ``propellant_mass``
     - ``.eng`` の値
     - 推進剤質量 [kg] を上書きする

``[launch]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``latitude`` / ``longitude``
     - 必須
     - 射点の緯度・経度 [deg]（WGS84）
   * - ``altitude``
     - 必須
     - 射点の標高 [m]
   * - ``rail_length``
     - 必須
     - ランチャレールの長さ [m]
   * - ``elevation_deg``
     - 必須
     - 射角（水平面からの仰角）[deg]
   * - ``azimuth_deg``
     - 必須
     - 射方位（北から時計回り）[deg]

``[recovery]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``enabled``
     - 必須
     - パラシュートを使うか。``false`` のとき落下分散は弾道落下のみ計算する
   * - ``cd_s``
     - 必須
     - パラシュートの抗力面積 :math:`C_D S` [m\ :sup:`2`]
   * - ``delay``
     - 必須
     - 頂点から開傘までの遅れ [s]

``[aero]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``method``
     - ``"barrowman"``
     - 空力係数の求め方。``"barrowman"`` は STL から抽出した形状に対する部品積み上げ法（:doc:`aerodynamics`）、
       ``"table"`` は外部の係数表の読み込み、``"panel"`` はパネル法（:doc:`panel`）
   * - ``table``
     - なし
     - ``method = "table"`` のとき必須。係数表の CSV ファイル（形式は後述）
   * - ``mach_min`` / ``mach_max`` / ``mach_step``
     - ``0`` / ``3`` / ``0.02``
     - 係数表の Mach 数の範囲と刻み
   * - ``alpha_max_deg`` / ``alpha_step_deg``
     - ``30`` / ``1``
     - 係数表の迎角の範囲（0 から）と刻み [deg]
   * - ``n_slices``
     - ``600``
     - 形状抽出で STL を切る断面の数
   * - ``fin_threshold``
     - ``0.05``
     - フィン判定のしきい値（最大胴体半径に対する比）
   * - ``extrapolation``
     - ``"linear"``
     - 表の範囲外の扱い: ``"linear"`` （線形外挿）または ``"clamp"`` （端の値で一定）

``[aero.panel]``\ （``method = "panel"`` のときだけ使う）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``body_axial`` / ``body_circ``
     - ``80`` / ``48``
     - 胴体のパネルの軸方向・周方向の分割数（軸方向にはフィン根元の翼弦の分割を含む）
   * - ``fin_chord`` / ``fin_span``
     - ``16`` / ``10``
     - フィン片面のパネルの翼弦方向・スパン方向の分割数
   * - ``wake_length``
     - ``20.0``
     - フィン後流の長さ（全長に対する倍率。全長の 0.5 倍以上）
   * - ``tail_radii``
     - ``6.0``
     - 底面を塞ぐ尾部フェアリング（円筒 + 半球）の円筒部の長さ（底面半径の倍数）。0 なら開口（非推奨）
   * - ``subsonic_machs``
     - ``[0, 0.3, 0.5, 0.6, 0.7, 0.8]``
     - 亜音速のパネル解を計算する Mach 数（1 未満。この間は pchip で内挿）
   * - ``transonic``
     - ``[0.8, 1.2]``
     - 遷音速の補間区間の Mach 数の範囲（下端は最後の亜音速解の Mach 数に合わせる）
   * - ``fin_section``
     - ``"biconvex"``
     - フィンの断面形状（現在は両凸のみ）

各項目の意味と、パネル法が出力する ``panel_cp.csv`` は :doc:`panel` で説明する。

``[earth]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"flat"``
     - 積分する座標系。``"flat"`` は射点の局所 ENU 座標系（平面地球・慣性系）、
       ``"ecef"`` は WGS84 楕円体に固定した回転座標系（Coriolis 力と遠心力を含む。:ref:`sec-ecef`）
   * - ``gravity``
     - ``"inverse_square"``
     - 重力モデル。``"constant"``\ （:math:`g_0` 一定）、``"inverse_square"``\ （逆 2 乗則）、
       ``"j2"``\ （点質量 + :math:`J_2` 項。:ref:`sec-gravity`）。``"j2"`` は ``model = "ecef"`` のときだけ指定できる

``[atmosphere]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"us1976"``
     - ``"us1976"``\ （米国標準大気 1976）または ``"constant"``\ （高度によらず一定）
   * - ``temperature_offset``
     - ``0.0``
     - 標準大気の温度に一律に足す温度差 :math:`\Delta T` [K]（``us1976`` のみ。:math:`-200` K より大きい値）
   * - ``density``
     - ``1.225``
     - 密度 [kg/m\ :sup:`3`]（``constant`` のみ）
   * - ``sound_speed``
     - ``340.29``
     - 音速 [m/s]（``constant`` のみ）
   * - ``viscosity``
     - ``1.789e-5``
     - 粘性係数 [Pa s]（``constant`` のみ）

``constant`` では三つの値がいずれも正でなければならない。この設定は飛翔計算だけでなく、
摩擦抗力の Reynolds 数（射点高度の大気）にも使われ、係数表の入力ハッシュにも含まれる。

``[wind]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"power"``
     - 風速の高度分布。``"constant"``\ （一定風）、``"power"``\ （べき法則）、``"log"``\ （対数則）、
       ``"profile"``\ （高度別の表。:ref:`sec-wind`）
   * - ``speed``
     - ``4.0``
     - 基準高度での風速 [m/s]（0 以上）。``profile`` では使わず、表から求めた基準高度の値で置き換える
   * - ``direction_deg``
     - ``0.0``
     - 風が **吹いてくる** 方位（北から時計回り）[deg]。``profile`` では使わない
   * - ``ref_height``
     - ``2.0``
     - 風速の基準高度 [m]（正）
   * - ``exponent``
     - ``6.0``
     - べき法則の指数 :math:`n`\ （正。``power`` のみ）
   * - ``roughness_length``
     - ``0.03``
     - 粗度長 :math:`z_0` [m]（``log`` のみ。0 より大きく ``ref_height`` より小さい値）
   * - ``profile``
     - なし
     - ``model = "profile"`` のとき必須。高度別の風の CSV ファイル（形式は :ref:`sec-wind-profile`）

``[sim]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``integrator``
     - ``"rk4"``
     - 数値積分法。``"rk4"``\ （固定刻みの古典的 Runge–Kutta 法）``"rk45"``\ （刻み幅制御付きの Dormand–Prince 法。:ref:`sec-integrators`）、``"dop853"``\ （刻み幅制御付きの 8 次の Dormand–Prince 法）のいずれか
   * - ``attitude``
     - ``"normalize"``
     - 姿勢の更新法。``"normalize"`` は四元数を通常の状態量として積分し、各ステップ後に正規化する。``"lie_group"`` は SO(3) 上の Runge–Kutta–Munthe-Kaas 法で、四元数のノルムが丸め誤差の範囲で 1 に保たれる。どの積分法とも組み合わせられる
   * - ``dt``
     - ``0.002``
     - ``rk4`` の時間刻み [s]。``rk45``・``dop853`` では最初の刻み幅として使う
   * - ``rtol`` / ``atol``
     - ``1e-7`` / ``1e-6``
     - ``rk45``・``dop853`` の相対許容誤差・絶対許容誤差（いずれも正）
   * - ``max_time``
     - ``1200``
     - 計算を打ち切る時刻 [s]
   * - ``output_interval``
     - ``0.05``
     - ``trajectory.csv`` に書き出す間隔 [s]。``rk45`` では刻み幅の上限でもある
   * - ``descent``
     - 自動
     - ``sim`` の降下モード。省略時はパラシュートが有効なら ``"parachute"``、無効なら ``"ballistic"``

``[dispersion]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``mode``
     - ``"wind_grid"``
     - ``"wind_grid"``\ （風速 × 風向の格子）または ``"monte_carlo"``\ （:doc:`dispersion`）
   * - ``wind_speeds``
     - ``[1, 2, …, 7]``
     - ``wind_grid`` で計算する基準風速の一覧 [m/s]
   * - ``directions``
     - ``8``
     - ``wind_grid`` の風向の分割数（北から等間隔。1 以上）

``[dispersion.monte_carlo]``\ （``mode = "monte_carlo"`` のときに使う）

各ばらつきは互いに独立な正規分布 :math:`N(0,\sigma^2)` から引き、以下の値は標準偏差 :math:`\sigma` である。
0 を指定するとそのばらつきを無効にする。負の値や有限でない値はエラーになる。
「相対」は公称値に対する割合（0.03 なら 3 %。公称値に :math:`1+\text{乱数}` を掛け、下限は 0.05）、
「絶対」は公称値に加える量（単位は項目ごとに示す）である。

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``samples``
     - ``1000``
     - サンプル数（1 以上）
   * - ``seed``
     - ``1``
     - 乱数の種。サンプル :math:`i` は ``seed + i`` を種とする
   * - ``thrust_scale``
     - ``0.03``
     - 相対。推力（全力積が比例して変わる。推進剤質量は変えない）
   * - ``burn_time_scale``
     - ``0.02``
     - 相対。燃焼時間の伸縮（推力曲線の時間軸を伸ばす）
   * - ``dry_mass``
     - ``0.1``
     - 相対。乾燥質量（慣性モーメントも同じ比で変わる）
   * - ``cg``
     - ``0.01``
     - 絶対 [m]。乾燥重心の位置（正が後方）
   * - ``cn_scale``
     - ``0.10``
     - 相対。法線力係数 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰
   * - ``ca_scale``
     - ``0.15``
     - 相対。軸力（抗力）係数 :math:`C_A`\ （燃焼中・燃焼後）
   * - ``elevation_deg``
     - ``0.5``
     - 絶対 [deg]。ランチャの射角
   * - ``azimuth_deg``
     - ``1.0``
     - 絶対 [deg]。ランチャの射方位
   * - ``wind_speed``
     - ``1.0``
     - 絶対 [m/s]。基準風速（``[wind]`` の ``speed`` の周りに分布し、0 未満は 0 にする）
   * - ``wind_direction_deg``
     - ``15.0``
     - 絶対 [deg]。風向（``[wind]`` の ``direction_deg`` の周りに分布する）
   * - ``parachute_cd_s_scale``
     - ``0.1``
     - 相対。パラシュートの抗力面積 :math:`C_DS`

各項目の扱いの詳細は :ref:`sec-mc` で述べる。

.. _sec-aero-table-format:

外部の係数表（``aero.method = "table"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

風洞試験や CFD などで得た係数を使うときは、``aero.method = "table"`` と ``aero.table = "cfd_table.csv"`` を指定する。
STL からの形状抽出と空力モデルの計算は行わず、``sim`` と ``dispersion`` はこの表をそのまま使う（``aero`` は表の行数を表示するだけである）。
形式は ``out/aero_table.csv`` と同じで、CSV と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つが必要である。

CSV の 1 行目は次の見出しで、続く行は Mach 数が外側、迎角が内側の順（Mach 数ごとに迎角を小さい方から並べる）に、全格子点を 1 行ずつ書く。

.. code-block:: text

   mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2

各列は、法線力係数 :math:`C_N`、軸力係数（燃焼中・燃焼後）、圧力中心 :math:`x_{cp}` [m]（ノーズ先端から）、
:math:`C_{N\alpha}` [1/rad]、ピッチ減衰の和 :math:`S_0, S_1, S_2`\ （式 :eq:`eq-moment-damp`）である。

付随の ``.json`` には、基準面積 ``ref_area`` [m\ :sup:`2`]、基準直径 ``ref_diameter`` [m]、全長 ``length`` [m]、
格子 ``machs`` と ``alphas_deg``\ （各列の値）、``extrapolation``\ （``"linear"`` または ``"clamp"``）、``source_hash``\ （任意の文字列でよい）を書く。
行数が ``machs`` と ``alphas_deg`` の積に一致しないとエラーになる。``motor.aft_x`` を省略したときのモータ後端位置は、この ``length`` になる。

設定ファイルの未知の項目は、綴りの誤りを見逃さないようにすべてエラーとして扱う。

出力ファイル
------------

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - ファイル
     - 内容
   * - ``geometry.json``
     - 抽出した形状（全長、基準直径、ノーズ、ボートテール、フィン、断面ごとの半径など）
   * - ``profile.csv``
     - 断面ごとの胴体半径 ``r`` とフィン高さ ``fin_span``
   * - ``aero_table.csv`` / ``.json``
     - (Mach, 迎角) ごとの :math:`C_N`, :math:`C_A`\ （燃焼中・燃焼後）, :math:`x_{cp}`, :math:`C_{N\alpha}`、ピッチ減衰の和。``.json`` は格子・基準量・入力ハッシュ
   * - ``aero_drag.csv``
     - Mach 数ごとの抗力の内訳、:math:`C_{N\alpha}`、:math:`x_{cp}`\ （``method = "panel"`` では内訳なしで合計の ``cd_off``, ``cd_on`` のみ）
   * - ``panel_cp.csv``
     - ``method = "panel"``。表面パネルの位置・法線・面積・圧力係数 ``cp``\ （迎角 4°、最も低い亜音速 Mach 数）と部位（``body`` または ``fin``）。
       ``python/plot.py panel`` で作図する
   * - ``trajectory.csv``
     - 時刻歴（位置、緯度経度、速度、Mach 数、迎角、推力、質量、重心、圧力中心、安定余裕、動圧、姿勢、角速度）
   * - ``summary.json``
     - ランチャ離脱速度、最大 Mach 数、最大動圧、頂点、最小安定余裕、着地点など
   * - ``ignisyeet.kml``
     - ``kml = true`` のとき ``sim`` と ``dispersion`` が書き出す、Google Earth 用の KML ファイル 1 つ。射点、飛翔経路（段階ごとに色分け）とイベントの印、風の格子の着地点の輪郭、モンテカルロの着地点と誤差楕円をフォルダに分けて収める（:ref:`sec-kml`）
   * - ``dispersion.csv``
     - ``mode = "wind_grid"``。風の各ケースの着地点（ENU と緯度経度）、頂点、飛行時間など
   * - ``dispersion_mc.csv``
     - ``mode = "monte_carlo"``。サンプルごと・降下モードごとの 1 行。サンプル番号、降下モード、``status``\ （``ok`` または ``failed``）、
       引いたばらつき（相対のものは割合、``wind_speed_delta`` と ``wind_direction_deg_delta`` は風速・風向のずれ）、
       そのサンプルの基準風速・風向、着地点（ENU と緯度経度）、着地距離、着地時刻、着地速度、頂点、頂点時刻、最大 Mach 数、
       ランチャ離脱速度、最小静安定余裕。失敗した行は ``status`` 以降の結果が空欄になる
   * - ``dispersion_summary.json``
     - ``mode = "monte_carlo"``。降下モードごとの有効サンプル数 ``n``、失敗数 ``failed``、着地点の平均・共分散行列、
       :math:`1\sigma` と :math:`3\sigma` の誤差楕円（長半径・短半径・長軸の方位）、最大着地距離とそのサンプル番号、頂点高度の平均と標準偏差

.. _sec-kml:

Google Earth で見る（KML）
--------------------------

``[output]`` の ``kml`` が ``true``\ （既定）のとき、``sim`` と ``dispersion`` は ``ignisyeet.kml`` を、CSV や JSON と同じ出力先ディレクトリに書き出す。
KML は地理情報を記述する XML 形式で、OGC の標準（KML 2.2 :cite:`ogckml`）である。Google Earth（デスクトップ版・Web 版）や QGIS などで開くと、機体の経路や着地点を衛星写真の上に重ねて確認できる。
不要なときは ``kml = false`` にすればよい。各要素をクリックすると、時刻・速度・着地点などの数値が説明欄に表示される（説明欄の文は英語である）。

``ignisyeet.kml`` は 1 つの KML 文書で、次のフォルダ（Google Earth の左のパネルでチェックボックスにより表示を切り替えられる）からなる。
``sim`` と ``dispersion`` は別のコマンドなので、どちらも実行のたびにこのファイルを書き直す。
そのコマンドで計算した結果に加え、同じ出力先ディレクトリにある他の結果（``trajectory.csv`` と ``summary.json``、``dispersion.csv``、``dispersion_mc.csv`` と ``dispersion_summary.json``）を読み戻して、1 つにまとめる。
該当するファイルがなければ、そのフォルダは作らない（読めないファイルがあるときは、そのフォルダを省いたことを端末に表示する）。
したがって、``sim`` のあとに ``dispersion`` を実行すれば、飛翔経路と落下分散を同じ画面で重ねて見られる。
読み戻す結果は、以前に同じディレクトリへ書いたものである。設定を変えたときは、``sim`` と ``dispersion`` の両方を実行し直すこと。

``Launch site``
   射点を示す白いピンで、地面に固定して表示する（射角・方位・レール長を説明欄に載せる）。

``Flight``
   1 回の飛翔（``sim`` で選んだ降下モード）の内容で、``sim`` の結果（``trajectory.csv`` と ``summary.json``）があるときに入る。

   * **飛翔経路**：段階ごとに別の線にして色分けする。黄がランチャ滑走、赤が燃焼中、青が燃焼後の慣性飛行、緑がパラシュート降下である。
     線だけを描き、地面への塗りつぶしはしない。
   * **イベントの印**：ランチャ離脱（Rail exit。そのときの静安定余裕を含む）、燃焼終了（Burnout）、頂点（Apogee）、開傘（Parachute deploy。パラシュート降下のときだけ）、着地（Landing）。
     各印の説明欄に時刻・高度・速度などを載せる。着地の印は地面に固定し、射点からの距離と東・北方向の距離、着地速度を載せる。
   * フォルダの説明欄に、風の条件と色の凡例を載せる。

``Dispersion – wind grid``
   風の格子の結果（``dispersion.csv``）があるときに入る。降下モード（``ballistic`` と ``parachute``）ごとのフォルダの下に、風速ごとのフォルダがある。
   風速ごとに、同じ風速で風向を変えた着地点を風向の順に結んだ閉じた折れ線（輪郭）と、各ケースの着地点の印（風速と風向を名前にする）が入る。
   弾道落下は青、パラシュート降下は橙で、風速が大きいほど濃い色にする。

``Dispersion – Monte Carlo``
   モンテカルロの結果（``dispersion_mc.csv`` と ``dispersion_summary.json``）があるときに入る。降下モードごとのフォルダに、平均着地点（Mean landing）、:math:`1\sigma` と :math:`3\sigma` の誤差楕円（式 :eq:`eq-mc-stat` の共分散から作る 72 頂点の閉じた折れ線で、塗りつぶさない）、
   各サンプルの着地点（``Landing points`` フォルダ）が入る。失敗したサンプルは含まない。

落下分散の図形はすべて地面に固定して描く。

**高度の扱い。** 飛翔経路の高度は ``absolute``\ （絶対高度）で書き出す。値は WGS84 楕円体からの高さで、射点で ``launch.altitude`` から始まり、
飛翔中は ``launch.altitude`` に射点からの上向きの高さを足したものになる（ECEF モードでは楕円体高そのもの）。
KML の絶対高度は名目上は平均海面（EGM96 ジオイド）からの高度だが、IgnisYeet はジオイド高（楕円体と平均海面の差）を **扱わない**。
場所によっては数十 m の差になるので、``launch.altitude`` には射点の **海抜標高** を入れ、地形の高さと合っていることを確かめてほしい。
合っていなければ、経路が地面にめり込んだり浮いたりして見える。射点・着地点・誤差楕円など地面に固定する図形は、Google Earth の地形に沿って表示される。

ディレクトリ構成
----------------

.. code-block:: text

   Cargo.toml            Rust ワークスペース
   crates/geom/          STL の入出力、機軸の決定、断面切り出し、フィン抽出、サンプル形状
   crates/aero/          標準大気、空力モデル、係数表（保存・内挿・外挿）
   crates/panel/         パネル法の空力解析（亜音速 Morino 法、超音速局所傾斜法）
   crates/sim/           推力曲線、6 自由度飛翔、測地座標変換
   crates/cli/           ignisyeet コマンドと設定ファイル
   python/plot.py        作図（uv プロジェクト）
   examples/             サンプルの設定・STL・推力曲線（推力曲線は架空のもの）
   doc/                  このドキュメント（Sphinx）

.. _sec-doc-build:

ドキュメントのビルド
--------------------

このドキュメント（Sphinx）は ``doc/`` にある。図は TikZ（LuaLaTeX + dvisvgm）と matplotlib で作り、
matplotlib の図に使うデータは本体を実行して作る（``make data``。最初に ``cargo build --release`` も走る）。
``make html`` と ``make pdf`` はどちらも、これらの図とデータを作り直してから文書を生成する。

.. code-block:: sh

   cd doc
   make html             # HTML を生成する。build/html/index.html を開く
   make pdf              # PDF を生成する。doc/build/latex/ignisyeet.pdf ができる

Python の環境は uv でも pip でも用意できる。Makefile は既定では ``uv run --project .`` 経由で
``sphinx-build`` と ``python`` を呼ぶ（初回に依存パッケージが自動で入る）。

.. code-block:: sh

   # uv の場合（そのまま実行する）
   make html

   # pip の場合: 仮想環境に入れて、PY を空にすると有効な環境の sphinx-build / python をそのまま呼ぶ
   python3 -m venv .venv && . .venv/bin/activate
   pip install -r doc/requirements.txt
   make html PY=
   make pdf  PY=

``doc/requirements.txt`` には Sphinx、furo、sphinxcontrib-bibtex と、図の作成に使う numpy、pandas、matplotlib が入っている。

必要なソフトウェアは次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - ソフトウェア
     - 用途
   * - Rust（``cargo``）
     - 図のデータを作る本体のビルドと実行
   * - LuaLaTeX（luatexja）と dvisvgm
     - TikZ 図（HTML 用の SVG と PDF 用の PDF）
   * - inkscape
     - ロゴ（SVG）を PDF に変換する（PDF 版のみ）
   * - upLaTeX、dvipdfmx、latexmk
     - PDF 版の組版。TeX Live なら ``texlive-lang-japanese`` など日本語関連のパッケージを含めて入れる
   * - IPAex フォント
     - 図と PDF 本文の日本語

PDF の組版中に LaTeX がエラーで止まったときは、``doc/build/latex/ignisyeet.log`` を開き、``!`` で始まる行とその直後の行番号を確認する。
多くはパッケージ不足かフォント不足である。原因を直したあと、もう一度 ``make pdf`` を実行すればよい。
