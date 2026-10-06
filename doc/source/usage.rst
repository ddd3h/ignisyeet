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

IgnisYeet は、計算の重い部分を `Rust <https://www.rust-lang.org/>`__、作図を `Python <https://www.python.org/>`__ で書いている。二つの言語の特徴と、この役割分担にした理由を簡単に述べる。

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

インストーラ
~~~~~~~~~~~~

Linux（x86_64、aarch64）と macOS（Intel、Apple Silicon）では、インストーラ ``install.sh`` を実行するだけでインストールできる。
ここでは、利用者（user）として入れる場合の手順を、実際の画面とともに順に説明する。
画面は ``install.sh`` の実際の出力であり、パスはホームディレクトリを ``~`` で示している。

手順 1　準備
^^^^^^^^^^^^

必要なものは次のとおりである。管理者権限（``sudo``）は要らない。すべて自分のホームディレクトリの下（既定は ``~/.local``）に入る。

* 対応する OS（上記の Linux または macOS）
* ``bash``、``curl``、``tar``\ （ほとんどの環境に最初から入っている）
* インターネット接続（リリースの取得と、作図用ライブラリの導入に使う）

``git``、``cargo``、``uv``、``python3`` などは、user では必須ではない。
``python3`` が 3.10 未満または無いときは、インストーラが ``uv`` を使って用意することを提案する。

手順 2　インストーラを実行する
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

次の 1 行を端末に貼り付けて実行する。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash

インターネット上のスクリプトをそのまま実行することに抵抗があるときは、いったん保存して中身を読んでから実行するとよい。

.. code-block:: sh

   curl -fsSLO https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh
   less install.sh
   bash install.sh

インストーラは起動すると、ロゴを表示してから質問を始める。
``curl ... | bash`` で実行しても、質問は端末から読み取るので、そのまま答えられる。

手順 3　役割を選ぶ
^^^^^^^^^^^^^^^^^^

最初に、どのように使うかを尋ねられる（:numref:`fig-screen-install-role`）。
``1`` が user（既定）、``2`` が developer である。Enter だけを押すと user になる。

user
   GitHub のリリースから自分のプラットフォーム用のバイナリを取得し、SHA-256 を検証してから
   ``~/.local/bin/ignisyeet`` に置く。例題と ``python/plot.py`` は ``~/.local/share/ignisyeet/<バージョン>`` に入り、
   ``current`` というリンクが最新版を指す。作図用の環境（uv、なければ ``venv`` と ``pip``）と、
   それを使う ``ignisyeet-plot`` コマンドも用意する。ドキュメント作成用のツールは入れない。

developer
   ソースからビルドして開発する人向けである。詳しくは後の「developer の場合」で述べる。

.. _fig-screen-install-role:

.. figure:: _generated/screens/screen_install_role.png
   :width: 100%
   :alt: インストーラの起動画面と役割の質問

   手順 3：インストーラの起動画面。``1``\ （user）を選んだところである。

手順 4　CFD ツールを入れるか選ぶ
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

インストーラは、すでに入っているツール（cargo、uv、python3、git、conda や micromamba、既存の ``ignisyeet-cfd`` 環境、SU2、gmsh など）を検出して
一覧にする（``Environment``）。続けて、CFD ツール（SU2、gmsh、Open MPI。conda-forge から約 3 GB）を入れるかを尋ねられる
（:numref:`fig-screen-install-cfd`）。これは CFD による空力計算（:doc:`cfd` を参照）で使う任意の部品で、
Barrowman 法やパネル法だけを使うなら要らない。既定は ``n``\ （入れない）である。
既存の micromamba、mamba、conda と既存の ``ignisyeet-cfd`` 環境があればそれを使い、すでに SU2 が見つかっているときは質問しない。
質問を省くには ``--cfd`` または ``--no-cfd`` を指定する。

.. _fig-screen-install-cfd:

.. figure:: _generated/screens/screen_install_cfd.png
   :width: 100%
   :alt: 環境の検出結果と CFD ツールの質問

   手順 4：検出したツールの一覧と CFD ツールの質問。``n`` と答えたところである。

手順 5　計画を確認する
^^^^^^^^^^^^^^^^^^^^^^

答えがそろうと、実行する内容が ``Plan`` として一覧になり、``Proceed?`` と尋ねられる（:numref:`fig-screen-install-plan`）。
この時点では何も変更していないので、内容が気に入らなければ ``n`` で中止できる。

``Environment`` の行頭の記号は、次の意味である。

* 緑のチェック（✔）：すでに見つかった。入っているものは再インストールせず、そのまま使う。
* 黄色の丸（○）：見つからなかった。必要なものは ``Plan`` で入れる手順に含まれる。

``Plan`` には、取得するファイル名、置き場所、作る環境が順番に並ぶ。
``y`` または Enter で進む。

.. _fig-screen-install-plan:

.. figure:: _generated/screens/screen_install_plan.png
   :width: 100%
   :alt: 実行計画の一覧と確認

   手順 5：実行計画と確認。``y`` と答えて実行に進む。

手順 6　インストールの進行を見る
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

各手順は、スピナー（回転する記号）とともに実行される。ダウンロードでは、進み具合を示すバーとサイズが出る
（:numref:`fig-screen-install-progress`）。続いてチェックサムの検証、展開、作図用環境の作成が行われ、
終わった手順には緑のチェックと所要時間が付く。
詳しい出力はすべて ``~/.local/share/ignisyeet/install.log`` に記録される。
作図用環境の作成は、初回はライブラリ（numpy、pandas、matplotlib）の取得のため、数十秒かかることがある。
ライブラリの版は、リリースに含まれる ``uv.lock``\ （uv を使う場合）または ``plot-constraints.txt``\ （pip を使う場合）で固定してある。

.. _fig-screen-install-progress:

.. figure:: _generated/screens/screen_install_progress.png
   :width: 100%
   :alt: ダウンロード中の進行バー

   手順 6：ダウンロード中の画面。進行バーと、取得済みのサイズ・全体のサイズが出る。

手順 7　完了と PATH の設定
^^^^^^^^^^^^^^^^^^^^^^^^^^

最後に、結果をまとめた枠（``IgnisYeet is ready``）が出る（:numref:`fig-screen-install-done`）。
インストールされたバージョン、実行ファイル、データ、作図環境の場所と、次に実行するコマンドが書かれている。

実行ファイルを置く ``~/.local/bin`` が ``PATH`` に入っていないときは、インストーラがシェルの設定ファイル
（bash なら ``~/.bashrc``、zsh なら ``~/.zshrc``）に PATH の行を追加するか尋ねる。
``y`` と答えるか、最初から ``--add-path`` を付けると、次の行が追記される。
``n`` と答えたときは、枠の下に表示される行を、自分で設定ファイルに書き加える。

.. code-block:: sh

   export PATH="$HOME/.local/bin:$PATH"

設定ファイルを書き換えたあとは、**新しい端末を開く**\ （または ``source ~/.bashrc`` を実行する）と、``ignisyeet`` コマンドが使えるようになる。

.. _fig-screen-install-done:

.. figure:: _generated/screens/screen_install_done.png
   :width: 100%
   :alt: インストール完了の枠と PATH の案内

   手順 7：完了の画面。PATH の質問（ここでは ``n``）、結果の枠、``PATH`` に追加する行が出る。

手順 8　動作を確認する
^^^^^^^^^^^^^^^^^^^^^^

新しい端末で、バージョンを表示して確かめる。

.. code-block:: sh

   ignisyeet --version

続いて、インストールされた例題を作業用の場所にコピーし、計算を実行する。
計算が終わると ``out/`` に結果の CSV と JSON が書かれる（:numref:`fig-screen-install-check`、:numref:`fig-screen-install-check-sim`）。

.. code-block:: sh

   cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples
   cd ignisyeet-examples
   ignisyeet sim sample.toml

図を描くには、``ignisyeet-plot`` を使う。作図用の環境はインストーラが用意したものが自動で使われる。

.. code-block:: sh

   ignisyeet-plot all out

.. _fig-screen-install-check:

.. figure:: _generated/screens/screen_install_check.png
   :width: 100%
   :alt: ignisyeet --version の出力と例題の実行

   手順 8：``ignisyeet --version`` と、例題のコピーと実行。

.. _fig-screen-install-check-sim:

.. figure:: _generated/screens/screen_install_check_sim.png
   :width: 100%
   :alt: インストールした例題の計算結果

   手順 8：``ignisyeet sim sample.toml`` の出力。計算の流れ、飛翔の結果、書き出したファイルが出る。

計算全体の流れは、この後の「最初の実行」を参照されたい。

developer の場合
^^^^^^^^^^^^^^^^

手順 3 で ``2``\ （developer）を選ぶと、手順 6 の内容が次のように変わる。
その他の手順は user と同じである。

* git でリポジトリを取得する（既存の clone があれば再利用し、変更がなければ ``git pull`` する）。
* ``cargo`` がなければ ``rustup`` を入れ、``cargo build --release`` と ``cargo test --release`` を実行する（テストは実行するか尋ねられる）。
* ``python/`` の環境を ``uv sync --frozen``\ （``uv.lock`` の版をそのまま使う）、``doc/`` の環境を ``uv sync`` で作る。uv がなければ、``scripts/plot-constraints.txt`` で版を固定して ``pip`` で入れる。
* TeX Live、dvisvgm、mutool、inkscape、IPAex フォントのうち足りないものは、
  実行すべき ``apt`` または ``brew`` のコマンドを表示する（``sudo`` はインストーラ自身は実行しない）。

実行する内容は ``--dry-run`` で先に確かめられる（:numref:`fig-screen-install`）。

.. _fig-screen-install:

.. figure:: _generated/screens/screen_install.png
   :width: 100%
   :alt: install.sh の実行画面（developer、CFD あり、--dry-run）

   ``install.sh --developer --cfd --dry-run`` の画面。何も変更せずに、検出したツールと実行する手順の一覧だけを示す。

CFD ツールについて
^^^^^^^^^^^^^^^^^^

どちらの役割でも、CFD ツール（SU2、gmsh、Open MPI）を任意で入れられる（手順 4）。
CFD 環境のパッケージは、すべてのプラットフォームで SU2 8.3.0、gmsh 4.15.2（python-gmsh も同じ版）、Open MPI 5、Python 3.10 に固定する。
SU2 8.5.0 のパッケージは AVX-512 のない CPU で異常終了するため、8.3.0 を使う。
micromamba を新たに入れる場合は、版 2.9.0 を取得する。
インストール後に、設定ファイルの ``[aero.cfd] prefix`` に書く値（または環境変数 ``IGNISYEET_CFD_PREFIX``）を表示する。

オプション
^^^^^^^^^^

質問に答える代わりに、オプションで指定することもできる。
``curl ... | bash`` の場合は、オプションを ``bash -s --`` の後ろに付ける。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --developer --cfd -y

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - オプション
     - 内容
   * - ``--user`` ``--developer``
     - 役割を尋ねずに指定する
   * - ``--cfd`` ``--no-cfd``
     - CFD ツールを入れる、入れない
   * - ``--version vX.Y.Z``
     - 入れるリリース（既定は最新）。developer ではチェックアウトするタグ。形式は ``vX.Y.Z`` に限り、``v`` のない値や他の形式はエラーにする
   * - ``--prefix DIR``
     - インストール先（既定は ``~/.local``）。相対パスや ``~`` で始まるパスは、起動時に絶対パスへ直す（リンクが壊れないようにするため）
   * - ``--dir DIR``
     - developer の clone 先（既定は実行した clone、なければ ``~/ignisyeet``）。``--prefix`` と同様に絶対パスへ直す
   * - ``--add-path``
     - ``PATH`` の行をシェルの設定ファイルに、尋ねずに追記する
   * - ``--force``
     - 同じバージョンが入っていても入れ直す
   * - ``-y`` ``--yes``
     - 質問せず既定値を使う（user、CFD なし、シェル設定ファイルは書き換えない）
   * - ``--dry-run``
     - 計画だけを表示して終了する
   * - ``--uninstall``
     - バイナリ、リンク、リリースデータを削除する。作図環境と CFD 環境は確認してから削除する（``--purge`` で両方）。developer の clone は明示的に確認したときだけ削除する

環境変数
^^^^^^^^

各オプションには環境変数も対応する。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 環境変数
     - 対応するオプションなど
   * - ``IGNISYEET_ROLE``
     - ``--user`` ``--developer``\ （``user`` または ``developer``）
   * - ``IGNISYEET_VERSION``
     - ``--version``
   * - ``IGNISYEET_PREFIX``
     - ``--prefix``
   * - ``IGNISYEET_DIR``
     - ``--dir``
   * - ``IGNISYEET_CFD``
     - ``--cfd``\ （``1``）、``--no-cfd``\ （``0``）
   * - ``IGNISYEET_YES``、``IGNISYEET_DRY_RUN``、``IGNISYEET_FORCE``、``IGNISYEET_ADD_PATH``
     - ``-y``、``--dry-run``、``--force``、``--add-path``\ （``1`` で有効）
   * - ``IGNISYEET_NO_COLOR``
     - 色を使わない（``NO_COLOR`` を設定しても同じになる）
   * - ``IGNISYEET_DOWNLOAD_BASE``、``IGNISYEET_REPO_URL``
     - リリースの取得元の URL、developer が clone する git の URL（ミラーや検証用）

``NO_COLOR`` を設定するか、端末でない場所へ出力すると、装飾のない行単位の出力になる。
端末でないとき（``-y`` を付けた場合やパイプ越しで入力がない場合を含む）は質問せず、既定値（user、CFD なし、PATH は書き換えない）を使う。
リリースの配布物は ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と ``.sha256`` であり、
インストーラはチェックサムを検証してから展開する。

アンインストール
^^^^^^^^^^^^^^^^

インストーラに ``--uninstall`` を付けて実行する。

.. code-block:: sh

   bash install.sh --uninstall

バイナリ、``ignisyeet-plot`` のリンク、リリースデータを削除するか尋ねたあと、
作図用の環境、CFD の conda 環境（作成していた場合）を削除するかをそれぞれ尋ねる（:numref:`fig-screen-install-uninstall`）。
developer の clone は、明示的に確認したときだけ削除する。作業中の変更が失われるので、通常は ``n`` と答える。
``--purge`` を付けると、作図環境と CFD 環境も尋ねずに削除する。
``install.log`` と、シェルの設定ファイルに追記した PATH の行は残るので、不要なら手で消す。

.. _fig-screen-install-uninstall:

.. figure:: _generated/screens/screen_install_uninstall.png
   :width: 100%
   :alt: install.sh --uninstall の実行画面

   ``install.sh --uninstall`` の画面。バイナリとリリースデータ、作図環境の削除に ``y`` と答えたところである。

更新と再インストール
^^^^^^^^^^^^^^^^^^^^

新しいバージョンに更新するには、手順 2 のコマンドをもう一度実行する。
入っていないものだけを入れるので、再実行しても安全である。
user では最新のリリースが取得され、``current`` のリンクが新しい版に切り替わる。古い版のデータは ``~/.local/share/ignisyeet/<バージョン>`` に残る。
特定のバージョンを入れたいときは ``--version vX.Y.Z`` を指定する。
同じバージョンがすでに入っているときは、``Reinstall it?`` と尋ねられる。尋ねられない場面（``-y`` など）で入れ直したいときは ``--force`` を付ける。

困ったとき
^^^^^^^^^^

``could not determine the latest release``\ （最新のリリースが分からない）
   まだリリースが公開されていないか、ネットワークに接続できていない。developer の役割（``--developer``）で入れるか、
   ``--version vX.Y.Z`` でバージョンを指定する。

``ignisyeet: command not found``
   ``~/.local/bin`` が ``PATH`` に入っていない。手順 7 の ``export PATH=...`` の行をシェルの設定ファイルに書き、新しい端末を開く。

プロキシの内側でダウンロードに失敗する
   ``curl`` は ``https_proxy`` と ``http_proxy`` を読む。実行前に設定しておく（例：``export https_proxy=http://proxy.example.com:8080``）。

途中で失敗した
   失敗した手順の名前と、ログの末尾が画面に出る。全体の記録は ``~/.local/share/ignisyeet/install.log`` にある。
   インストーラは再実行しても安全なので、原因を取り除いてからもう一度実行する。

CFD ツールで SU2 が異常終了する（AVX-512 のない CPU）
   AVX-512 のない CPU では、SU2 8.5.0 が異常終了する。
   インストーラが作る環境は SU2 8.3.0 に固定しているが、別の版がすでに入った環境を使っているときは、画面に出る再作成のコマンドで作り直す。

ソースからのビルド
~~~~~~~~~~~~~~~~~~

インストーラを使わずに、リポジトリから直接ビルドすることもできる。

.. code-block:: sh

   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   cargo build --release      # target/release/ignisyeet ができる

CFD モード（``aero.method = "cfd"``）は、Unix 系の OS（Linux、macOS、Windows の WSL）が前提である。
CFD 以外の部分は Windows でも ``cargo check`` が通るが、インストーラは Linux と macOS 向けである。

.. _sec-run-first:

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

``aero``、``sim``、``dispersion`` は空力係数表のキャッシュを共有するので、Makefile は ``make -j`` でも直列に実行する。
実行ファイルは ``$CARGO_TARGET_DIR/release/ignisyeet``\ （未設定なら ``target/release/ignisyeet``）で、``BIN=...`` で上書きもできる。設定ファイルと出力先のパスに空白があっても動く。

``make`` が図を読む出力ディレクトリ（``OUT``）は、設定ファイルの ``[output] dir``\ （設定ファイルのあるディレクトリからの相対パス。絶対パスならそのまま、未指定なら ``out``）から決まる。
``OUT=...`` で上書きもできる。``[output] dir`` は、基本文字列（``"..."``）でもリテラル文字列（``'...'``）でも読める。

``sim`` や ``dispersion`` が終了ステータス 3（結果は書いたが不完全。:ref:`sec-exit-status`）で終わっても、``make all`` は作図まで進める。
作図のあと、``results are INCOMPLETE (exit status 3)`` で始まるメッセージ（この 3 は ``ignisyeet`` の終了ステータス）を出して失敗する。
``make`` 自体の終了ステータスは、GNU make の規則で 2 になる。
ステータス 3 以外の失敗では、その場で止まる。

``python/plot.py`` は、出力ディレクトリがないとき、また描くものが一つもないときは、エラーを表示して終了ステータス 1 で終わる。
``all`` では、実際に図を書いたものだけを数える。個別の図（``geometry`` など）を選んで何も描けなかったときも、``nothing to plot for …`` というエラーで終了ステータス 1 になる。
``profile.csv`` がないと、形状の図は ``skip geometry: profile.csv not found (run `ignisyeet geom` again)`` と表示して描かない。
``method = "cfd"`` の出力には ``aero_drag.csv`` がない（:ref:`sec-file-drag`）ので、空力の図は、``aero_table.csv`` の迎角 0 の行から求めた全体の抗力係数で描く（抗力の内訳は含まない）。
一般表（``aero_table.csv`` が :math:`C_X` などの 6 成分の列を持つ）でも ``aero_drag.csv`` がない。このときは横滑り角 0 の面で、:math:`C_D=-C_X`、:math:`C_{N\alpha}=|C_Z|/\alpha`\ （最小の正の迎角）、:math:`x_{cp}=C_m\,d/C_Z`\ （:math:`d` は ``aero_table.json`` の基準直径）を Mach 数ごとに求めて描く。
``sim`` が失敗した出力ディレクトリ（``summary.json`` が失敗の印）では、``python/plot.py trajectory`` は ``error: the last sim run failed: …; not plotting the old trajectory`` で終了ステータス 1 になり、
``all`` は ``skip trajectory: the last sim run failed: …`` と表示して、飛翔経路の図だけを描かない（古い結果を描かない）。
``dispersion_mc.csv`` があって ``dispersion_summary.json`` がないときは、``skip dispersion_mc: dispersion_summary.json not found (run `ignisyeet dispersion` again)`` と表示して、モンテカルロの図を描かない。
``dispersion.csv`` などの落下分散の結果が、``summary.json`` と別の飛翔の設定で計算されたもの（``dispersion_meta.json`` と ``dispersion_summary.json`` の ``flight_config_hash`` が ``summary.json`` の ``config_hash`` と一致しない、またはない）なら、標準エラー出力に警告を出す。
風の格子の図では、風速ごとの線を、外側（風速が大きい）から順に並ぶ凡例とともに、viridis から取った区別しやすい色で描く。
``wind.model = "profile"`` の結果では、凡例の見出しが ``profile mean speed (outermost first)`` になる（それ以外は ``wind speed (outermost first)``）。
二段開傘の飛翔では、軌跡の図にメインの開傘の位置を四角の印で示す。

``python/plot.py`` は ``geometry`` ``aero`` ``trajectory`` ``dispersion`` のいずれかを選んで描くこともできる。
``dispersion`` は ``dispersion.csv`` があれば風の格子の図（``dispersion.png``）を、
``dispersion_mc.csv`` と ``dispersion_summary.json`` があればモンテカルロの図（``dispersion_mc.png``）を描く（両方あれば両方描く）。
``aero`` に ``--compare <別の出力ディレクトリ>`` を付けると、二つの係数表の :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{D0}` を Mach 数に対して重ねた図
（``aero_compare.png``。凡例はディレクトリ名）も描く。

.. _sec-python-env:

uv を使わずに ``pip`` で作図する場合は、仮想環境を作って依存パッケージを入れ、``python`` から直接実行する。

.. code-block:: sh

   python3 -m venv .venv && . .venv/bin/activate
   pip install -r python/requirements.txt -c scripts/plot-constraints.txt
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

二段開傘（``recovery.main_cd_s``）のときは、結果パネルにメインの開傘の高さと時刻の行（``main deploy``）を加え、設定パネルにはメインの抗力面積と開傘高度、ドローグとメインの終端降下速度（海面、乾燥質量）を示す。
パラシュートを使う設定の設定パネルには、終端降下速度の見積り（``terminal speed``）が並ぶ。
これは :math:`\sqrt{2mg/(\rho\,C_DS)}` に海面の標準密度 :math:`\rho=1.225\ \mathrm{kg/m^3}` と乾燥質量 :math:`m` を入れた値で、サンプルでは 6.0 m/s である。
飛翔の計算結果ではなく簡易な見積りなので、着地速度（地面に対する速さで、横方向の速度を含む。サンプルでは 7.1 m/s）とは一致しない。
結果パネルの静安定余裕の行（``min margin``）には、自由飛行中の最小値とランチャ離脱時の値の小さいほうを示す。自由飛行中の最小値は、ランチャ離脱の時刻（自由飛行はここから始まる）から頂点までのうち対気速度が 10 m/s を超える間の値で、ランチャ離脱の時刻の値を含む。プレーンモードでは ``min static margin X cal (free flight Y cal)`` の 1 行で、自由飛行中の最小値 ``Y`` も添える。

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
   * - 0 cal 以上 1.0 cal 未満
     - 赤の記号と ``low static margin``
     - 余裕が不足している。設計を見直す
   * - 0 cal 未満
     - 赤のバツ印と ``UNSTABLE``
     - 不安定（圧力中心が重心より前にある）。設計を見直す

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
       書き出したファイルは ``-> パス`` の行に並べる（``sim`` は ``trajectory.csv`` と ``summary.json``、``dispersion`` は結果の CSV と JSON）。
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
     - 設定した風で 1 回飛翔させ、``trajectory.csv`` と ``summary.json`` を書き出す。``sim.max_time`` までに着地しなかったとき、または静的に不安定なときは、結果を書いたうえで終了ステータス 3 で終わる（:ref:`sec-exit-status`）。
       実行の最初に、出力先の古い ``trajectory.csv`` と ``summary.json`` を消す。実行がエラーで終わったときは、``summary.json`` に失敗の印 ``{"failed": true, "error": "…"}`` だけを書く（:ref:`sec-file-summary`）。
   * - ``dispersion <config>``
     - ``dispersion.mode`` に従って落下分散を並列に計算する（保安距離の検討の手順は :ref:`sec-dispersion-safety`）。``"wind_grid"`` では風速 × 風向の全ケースを計算して ``dispersion.csv`` と ``dispersion_meta.json`` を、
       ``"monte_carlo"`` では各サンプルの結果 ``dispersion_mc.csv`` と統計 ``dispersion_summary.json`` を書き出す。
       着地しなかったケースや失敗したケースがあるとき、公称の飛翔が静的に不安定なとき、または静的に不安定なケースやサンプルがあるときは、結果を書いたうえで終了ステータス 3 で終わる。
       風の格子のすべてのケースが積分のエラーで止まったときは、``dispersion.csv`` などを書いたうえで終了ステータス 1 で終わる（:ref:`sec-wind-grid-failed`）。
   * - ``cfd-check <config> [--mesh]``
     - CFD に必要なツール（SU2、MPI、gmsh。``surface = "stl"`` では manifold3d も）の有無と版、ケース数を表示する。``--mesh`` は壁面のメッシュも作って品質を表示する（:doc:`cfd`）。
       必要なツールが足りないときは終了ステータス 1 で終わる。
   * - ``sample-stl <path>``
     - 付属のサンプル機体の STL（mm 単位、ノーズ +z 向き）を書き出す。

``sim`` と ``dispersion`` は、係数表がない場合や入力が変わった場合に自動で ``aero`` を実行する。
入力の同一性は、STL の内容と設定値から計算したハッシュ値（FNV-1a）で判定する。
ハッシュには、プログラムの版と、空力モデルの版（係数の計算方法が変わると上げる内部の番号）も入るので、計算方法が変わった版では保存済みの表を使わずに作り直す。
空力に使わない設定は、ハッシュに入れない。選んでいない手法のブロック（``method = "barrowman"`` のときの ``[aero.panel]`` と ``[aero.cfd]`` など）、
外部の係数表の指定、``method = "cfd"`` の資源とパスの項目（``prefix`` ``su2`` ``mpi`` ``ranks_per_case`` ``parallel_cases`` ``timeout_minutes`` ``workdir``）である。
これらを変えても表は作り直さない。

.. _sec-config-checks:

設定の検査と警告
----------------

設定ファイルは読み込み時に検査し、不正な値は項目名を示してエラー（終了ステータス 1）にする。
有限でない値（``nan`` や ``inf``）は、すべての数値項目で受け付けない。主な範囲は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 項目
     - 条件
   * - ``launch.latitude`` / ``launch.longitude``
     - :math:`-90` から :math:`90` / :math:`-180` から :math:`180`
   * - ``launch.elevation_deg``
     - :math:`0 <` 値 :math:`\le 90`
   * - ``launch.rail_length``、``rocket.stl_scale``、``rocket.dry_mass``、``rocket.ixx_dry`` ``iyy_dry`` ``izz_dry``
     - 正
   * - ``rocket.cg_dry``
     - 0 以上（ノーズ先端から後方に測る）
   * - ``rocket.roughness``、``rocket.extra_cd``、``recovery.delay``、``wind.speed``
     - 0 以上
   * - ``recovery.cd_s``
     - 0 以上。``recovery.enabled = true`` のときは正（0 ではパラシュートが減速しない）
   * - ``recovery.main_cd_s`` と ``recovery.main_altitude``
     - 両方を指定するか、両方とも省略する。指定するときは、どちらも有限で正
   * - ``aero.mach_min`` と ``aero.mach_max``
     - ``mach_min`` は 0 以上、``mach_max`` は ``mach_min`` より大きい
   * - ``aero.mach_step``、``aero.alpha_max_deg``、``aero.alpha_step_deg``、``aero.general_mach_step``
     - 正
   * - ``aero.mach_max``
     - 10 以下（ロケットの飛翔の範囲を外れる）
   * - ``aero.alpha_max_deg``
     - 180 以下
   * - 係数表の格子点の数（Mach 数の数 × 迎角の数）
     - 約 10\ :sup:`6` 以下。超えるときは ``aero.mach_step`` と ``aero.alpha_step_deg`` を大きくするか、``mach_max`` と ``alpha_max_deg`` を小さくする。
       部品積み上げ法の一般表（``method = "barrowman"`` かつ ``table_kind = "general"``）では、Mach 数の刻みに ``aero.general_mach_step`` を使い、メッセージも ``aero.general_mach_step`` を挙げる。
       ``table_kind = "general"`` では、外部の表（``method = "table"``）以外のすべての手法（部品積み上げ法、パネル法、CFD）が軸対称の表を一般表へ展開するので、
       軸対称の表（Mach 数 × 迎角）と一般表（Mach 数 × 迎角 × 横滑り角）のうち、点の多いほうで判定する（メッセージは ``the aero table grid would have about … points``）
   * - ``aero.n_slices``
     - 20 以上 10000 以下（形状抽出に必要な断面の数）
   * - ``wind.ref_height``、``wind.exponent``、``wind.roughness_length``、``wind.ground_blend_height``
     - 正
   * - ``wind.roughness_length``\ （``model = "log"``）
     - ``wind.ref_height`` の半分より小さい
   * - ``sim.dt``、``sim.max_time``、``sim.output_interval``、``sim.rtol``、``sim.atol``
     - 正
   * - ``sim.dt``
     - 0.05 s 以下（0.002 s が標準）
   * - ``sim.rtol``、``sim.atol``
     - 10\ :sup:`-3` 以下（10\ :sup:`-8` が標準）
   * - ``sim.output_interval``
     - ``sim.max_time`` 以下、かつ 10\ :sup:`-4` s 以上。``sim.max_time / sim.output_interval`` は 10\ :sup:`7` 以下（出力の行数の上限。超えるときは ``sim.output_interval`` を大きくする）
   * - ``wind.direction_deg``
     - 0 以上 360 以下（``wind.direction_deg must be between 0 and 360, found 400`` の形のエラー）。``model = "profile"`` の表の風向の列も、各行が 0 以上 360 以下
   * - ``dispersion.wind_speeds`` の各要素、``[dispersion.monte_carlo]`` の各標準偏差
     - 0 以上
   * - ``dispersion.wind_speeds``
     - 1 個以上 100 個以下の要素（空のリストはエラー）
   * - ``dispersion.directions``
     - 1 以上 360 以下
   * - 風の格子のケース数（``wind_speeds`` の数 × ``directions``）
     - 100000 以下（降下モードを数えない）
   * - ``dispersion.monte_carlo.samples``
     - 1 以上 10\ :sup:`6` 以下
   * - ``atmosphere.density`` ``sound_speed`` ``viscosity``\ （``model = "constant"``）
     - 有限で正
   * - ``atmosphere.temperature_offset``\ （``model = "us1976"``）
     - 有限で、−186.946 K より大きい（モデルの範囲で最も低い標準温度 186.946 K に足して、絶対温度が正になる値）

どのセクションも、未知のキーがあるとキーの名前を示してエラーにする（``rol_deg`` のような綴りの誤りが黙って無視されないようにするためである）。

入力ファイルが読めないときは、エラーにファイルのパスを示す（STL、モータの ``.eng``、風のプロファイル、外部の係数表とその付随ファイル）。
STL は、頂点の座標に有限でない値（``NaN`` や ``inf``）があると、三角形の番号を示してエラーにする。
バイナリ STL は、見出しに書かれた三角形の数に必要なバイト数よりファイルが短いと、``binary STL declares 3 triangles (needs 234 bytes) but the file has 191 bytes: the file is truncated or corrupt`` の形（数は例）で、宣言した数、必要なバイト数、実際のバイト数を示してエラーにする。
``.eng`` の検査は :ref:`sec-propulsion-eng` に、外部の係数表の検査は :ref:`sec-aero-table-format` にある。

さらに、形状を抽出したあとで、次の場合にエラーにする（``stl_scale`` や ``cg_dry`` の単位の誤りを、飛翔計算の前に見つけるためである）。

* ``rocket.stl_scale`` を掛けた機体の全長が 0.1 m から 20 m の範囲の外にある。メッセージは ``body length … m is outside the plausible 0.1-20 m range: check the STL units and rocket.stl_scale`` の形で、``stl_scale`` の現在の値と、mm 単位の STL なら ``0.001`` であることを添える。
* 乾燥重心 ``rocket.cg_dry`` が機体の全長より後方にある。
* モータが機体の外にある。モータ後端の位置 ``motor.aft_x`` がモータの長さ（``.eng`` の長さ）より前、または機体の全長より後方にある（1 mm の許容がある）。
  ``motor.aft_x`` を省略したときの既定値は機体の全長なので、モータが機体より長い場合にこの検査に掛かる。
  メッセージは ``motor.aft_x = 0.2 m puts the motor outside the body: the motor is … m long and the body … m, so motor.aft_x (the motor's aft end, measured aft of the nose tip) must be between … and … m`` の形である。
  省略したときは先頭が ``motor.aft_x (default: the body length … m) puts the motor outside the body`` になる。

積分が発散して状態に有限でない値が現れたときも、エラーにする（``integration diverged at t = … s; reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties``。:ref:`sec-integrators`）。外部の係数表や質量特性の誤りでも発散するので、メッセージはそれらの確認も促す。
値が有限でも、速さが 8 km/s を超えるか高さが 1000 km を超えたときは、発散とみなして同じ形のエラーにする。
モンテカルロ解析では、そのサンプルを失敗として数える。風の格子では、そのケースだけを失敗として記録し、ほかのケースの計算は続ける（:ref:`sec-wind-grid-failed`）。

UTF-8 の BOM（バイト順マーク）が先頭にあっても読める（設定ファイル、``.eng``、外部の係数表とその JSON、風のプロファイルの表）。

エラーにはしないが疑わしい点は、警告として表示する。警告は ``-q`` を含むすべての出力モードで出す。
プレーンモードと端末の表示では標準出力に、``-q`` では標準エラー出力に ``warning:`` の行で書く。
``sim`` では ``summary.json`` の ``warnings`` にも、``dispersion`` では ``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` にも同じ文を残す。
警告の内容は次のとおりである。

* 機体の直径が 0.005 から 3 m の範囲を外れている（``rocket.stl_scale`` の誤りが疑われる。mm 単位の STL なら ``0.001``）。``geom`` でも表示する。全長の範囲外は、STL から作る機体ではエラーである（上記）。``method = "table"`` の外部の表では、表の ``length`` と ``ref_diameter`` について、全長 0.1 から 20 m、直径 0.005 から 3 m の範囲外を警告にする。
* 静安定余裕（自由飛行中の最小値とランチャ離脱時の値の小さいほう。自由飛行中の最小値は、ランチャ離脱の時刻から頂点までのうち対気速度が 10 m/s を超える間に、積分のステップの始端ごとに測った値で、ランチャ離脱の時刻を含む）が、0 cal 未満（``UNSTABLE``。圧力中心が重心より前にある）、0 以上 1.0 cal 未満（``low static margin``。不安定とは呼ばない）、1.0 cal 以上 1.5 cal 未満（``marginal``。推奨の 1.5 cal を下回る）である。
* 風のモデルが ``profile`` で、頂点が表の最上端の高度より高い（``apogee … m is above the top of the wind profile (… m): the wind above … m is held at the top value``）。表より上の風は最上端の値で一定になる。
* 風のモデルが ``power`` または ``log`` で、頂点が地上 1000 m を超え、風速が 0 でない（``dispersion`` の風の格子では、最大の風速が 0 でないとき）。判定に使う風速は、実際にその飛翔で使う基準風速である（``profile`` では ``wind.speed`` ではなく表から求めた値）。これらのプロファイルは基準高さ ``wind.ref_height`` の風速 ``wind.speed`` を上空に外挿するので、上空の風と着地点はモデル依存になる（実際の予測には ``model = "profile"`` を使う）。
* 風のモデルが ``profile`` で、表の最下行が地上 0 m より上にある（``the wind profile starts at … m: the wind below is held at the lowest row``）。
* 風のモデルが ``profile`` の風の格子またはモンテカルロ解析で、表の拡大率 :math:`k`\ （:ref:`sec-wind-scaling`）が 3 を超えるか 1/3 を下回るケースがある。最も極端なケースの拡大率と、表の風速の最大値を拡大した値を示す（``wind profile scaled by up to … (wind grid … m/s vs profile mean … m/s): winds aloft up to … m/s``。モンテカルロ解析では ``wind grid`` が ``Monte Carlo sample`` になる）。
* パラシュート降下が着地した飛翔（``sim`` と、``dispersion`` の公称の飛翔）で、開傘しないまま着地した（``parachute descent landed at … m/s: the parachute did not deploy before landing (recovery.delay = … s)``）か、着地速度が 15 m/s を超える（``… which is above 15 m/s: check recovery.cd_s``）。
* ``sim.max_time`` までに着地しない。
* 二段開傘で、メインの開傘高度 ``recovery.main_altitude`` が頂点の高度以上である（``recovery.main_altitude = … m is above the apogee (… m): the main opens together with the drogue``）。
* ランチャ離脱速度が 15 m/s 未満である（``rail exit speed … m/s is below 15 m/s: the fins have little authority at rail exit and the rocket is easily blown off the rail line; use a longer rail or a higher thrust-to-weight ratio``）。
* ランチャ離脱の高さ（レール長 × sin 射角）の風速と離脱速度から求めた風による迎角 :math:`\arctan(W/v)` が 15° を超える（``the wind angle of attack at rail exit is … deg (wind … m/s at … m, rail exit speed … m/s), above 15 deg: the rocket will weathercock strongly off the rail``）。
  風速 :math:`W` は、``sim`` では設定の風、``dispersion`` の風の格子では格子の最大の風速、モンテカルロ解析では公称の風速に風速の標準偏差の 2 倍を足した値である。
  離脱速度は公称の飛翔の値である。
* モータの推力曲線の疑わしい点。最後の点が 0 N でない（``thrust curve ends at … N instead of 0 N: the motor is cut off abruptly``）、点が 1 つだけ（``the thrust curve has a single point: the thrust ramps linearly from 0 N at t = 0``）、燃焼時間が 0.05 s より短い（``burn time … s is shorter than 0.05 s: the motor is an impulse; check the time column``）。
* 外部の係数表（``method = "table"``）で、迎角 0 の軸力係数 ``ca_on`` または ``ca_off`` が負である（``aero table: ca_off at Mach …, alpha 0 is … (negative axial force / thrust-like drag)``）。列ごとに、最初の Mach 数を示す。
* ``dispersion`` で、静的に不安定なケースまたはサンプルがある。降下モードごとに数と最小の静安定余裕を示す（:ref:`sec-dispersion-unstable`）。
* モンテカルロ解析で、着地したサンプルが 30 未満の降下モードがある（``only … landed samples: the statistics are rough``）。
* ``dispersion`` の風の格子で、積分がエラーで止まったケースがある。最初の 3 ケースをエラーの文とともに表示し、残りは数だけを表示する（:ref:`sec-wind-grid-failed`）。
* 係数表の範囲外の引き出しがある。回数、最大の Mach 数、最大の :math:`|\alpha|` を ``aero table: 370854 of 10159590 lookups beyond the angle range (max Mach 1.51, max |alpha| 90.0 deg); Mach is held at the table edge, angles are extrapolated`` の形で表示する（``alpha_max_deg = 30`` にしたパネル法のサンプルの ``dispersion`` での表示。Mach 数の範囲外があるときは ``… lookups beyond the Mach range`` も並ぶ）。サンプルの設定 ``alpha_max_deg = 90`` では、表の範囲外の引き出しはなく、この警告は出ない。
  迎角の範囲外の引き出しは、頂点のあとの弾道落下（機体が反転し、尾部を先にして落ちる）で起きることが多く、その場合は想定内である。
  上昇中や燃焼中に起きるときは、結果に影響する。``trajectory.csv`` の迎角で時刻を確かめ、``aero.alpha_max_deg`` を大きくして（たとえば 90）係数表を作り直す。
  表の外では、フィンの項は失速せず線形のままなので、30° を超える範囲の係数は粗い（:ref:`sec-lim-aero`）。
* 形状の抽出結果の疑わしい点（:ref:`sec-geometry-warnings`）。``sim`` と ``dispersion`` を含むすべてのモードで表示し、係数表をキャッシュから読むときは、同じ出力先の ``geometry.json`` の ``warnings`` を使う。
* CFD モードで、SU2 のバージョンが 8.3.x でない。``cfd-check`` と CFD の係数表の作成で表示する。

.. _sec-exit-status:

終了ステータス
--------------

.. list-table::
   :header-rows: 1
   :widths: 15 85

   * - 値
     - 意味
   * - 0
     - 成功
   * - 1
     - エラー（不正な設定、入力ファイルがない、処理の失敗、``cfd-check`` で必要なツールが足りない、など）
   * - 2
     - コマンドラインの使い方の誤り
   * - 3
     - 出力は書いたが、結果が不完全、または使えない。次のいずれかのときである。

       * ``sim`` で ``sim.max_time`` までに着地しなかった。
       * ``dispersion`` で着地しなかった、または失敗したケース・サンプルがある（風の格子で積分がエラーで止まったケースを含む）。
       * 静的に不安定である（下記。``dispersion`` では静的に不安定なケースやサンプルがあるときも含む）。

       着地しなかった場合と失敗があった場合は、標準エラー出力に ``INCOMPLETE RESULTS`` で始まる行を書く。
       静的に不安定な場合は、``RESULTS NOT USABLE`` で始まる行を書く

静的に不安定とは、静安定余裕の最小値が 0 cal 未満のことである。
最小値は、自由飛行中の最小値とランチャ離脱時の値の小さいほうで、結果パネルの ``min margin`` と同じである（:ref:`sec-config-checks`）。
``sim`` は、着地したかどうかにかかわらず、不安定なら終了ステータス 3 で終わり、標準エラー出力に ``RESULTS NOT USABLE: the rocket is statically UNSTABLE (min static margin -0.23 cal)`` の形の行を書く（``-0.23`` は例）。
``dispersion`` は、公称の飛翔（設定した風での弾道落下）が不安定なときに、風の格子でもモンテカルロ法でも、出力を書いたうえで終了ステータス 3 で終わる。
公称の飛翔が安定でも、風の格子のケースやモンテカルロのサンプルに静的に不安定なものがあれば、同じく終了ステータス 3 で終わり、
``RESULTS NOT USABLE: N of M Monte Carlo samples (ballistic) are statically UNSTABLE (min static margin down to X cal): …`` の形の行を書く（風の格子では ``wind-grid cases``）。
不安定なケースとサンプルは、降下モードごとに数え、統計には入れたままにする（:ref:`sec-dispersion-unstable`）。
``summary.json``、``dispersion_meta.json``、``dispersion_summary.json`` の ``stable`` と ``static_margin_min_cal`` に、公称の飛翔の判定に使った値を残す。
``dispersion_meta.json`` と ``dispersion_summary.json`` の ``unstable`` と ``unstable_min_static_margin_cal`` には、降下モードごとの不安定なケースの数と最小の余裕を残す（:ref:`sec-file-summary`、:ref:`sec-file-dispersion`）。
不安定な飛翔の結果は、保安の判断に使わない。

着地しなかった飛翔は、着地点が存在しない。``summary.json`` の ``landed`` が ``false`` になり、着地に関する項目は ``null`` になる（:ref:`sec-file-summary`）。
着地しなかった結果は KML に入れない。``sim`` が着地しなかったときは、前回の飛翔が KML に残らないよう、飛翔の層を除いた ``ignisyeet.kml`` を書き直す（``output.kml = true`` のとき）。
モンテカルロ解析の誤差楕円は、着地したサンプルが 2 つ未満の降下モードでは KML に入れない。

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
     - 重い処理のメモリ予算 [GB]。``0`` は制限なし。パネル法は密行列に必要な量（パネル数 n に対しておよそ 16 n² バイト）が予算を超えると、計算を始める前にエラーで止まる。予算が ``0`` のときも、必要な量がその計算機の物理メモリを超えるなら、同じく計算の前にエラーで止まる（``the panel method needs about … GB for … panels (dense matrix), more than the … GB of memory of this machine``。パネルの設定を粗くする。物理メモリは Linux の ``/proc/meminfo`` から読むので、読めない環境では確かめない）。CFD は予算に収まるよう同時実行ケース数を減らす。負の値はエラー
   * - ``nice``
     - ``0``
     - プロセスの優先度を下げる値（0 から 19）。長時間の計算で他の作業を妨げたくないときに使う。SU2 や ``mpirun`` の子プロセスにも引き継がれる。Unix 以外のプラットフォームでは警告して無視する。範囲外の値はエラー

実際に使うスレッド数、メモリ予算、``nice`` は、起動時の設定パネルに表示する（プレーン表示では 1 行で示す）。

.. _sec-config-rocket:

``[rocket]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

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
     - 推進剤を除いたときのロール / ピッチ慣性モーメント [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。``izz_dry`` を省略するとヨーも ``iyy_dry`` に等しい。CAD がないときの見積もりは :ref:`下の注意 <sec-usage-inertia-estimate>` を参照
   * - ``izz_dry``
     - ``iyy_dry``
     - 推進剤を除いたときのヨー慣性モーメント [kg m\ :sup:`2`]
   * - ``ixy_dry`` / ``ixz_dry`` / ``iyz_dry``
     - ``0``
     - 慣性テンソルの非対角要素 [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。**テンソルの行列要素** で、:math:`I_{xy}=-\int xy\,\mathrm{d}m`（:eq:`eq-inertia-tensor`）。
       CAD の慣性乗積の表示は符号が逆のことが多いので、確認すること（下の注意）
   * - ``cg_lateral_dry``
     - ``[0, 0]``
     - 乾燥重心の機軸からの横ずれ ``[y, z]`` [m]（機体座標）
   * - ``fin_cant_deg``
     - ``0``
     - フィンのカント [deg]。正のカントは :math:`+x_b` まわりの正のロールモーメントを生む。``fin_override.cant_deg`` があればそちらを使う
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
     - フィンの自動検出結果を上書きする。``count`` ``root_chord`` ``tip_chord`` ``span`` ``sweep`` ``thickness`` ``x_le_root`` ``cant_deg`` を必要なものだけ書く

.. note::

   **機体座標と慣性乗積の符号。** 慣性テンソル、``cg_lateral_dry``、``misalignment_offset`` は、機体座標（:math:`x_b` はノーズ方向）の値で与える。
   :math:`y_b`, :math:`z_b` の向きは STL の座標から決まり、``geometry.json`` の ``body_x`` ``body_y`` ``body_z``\ （STL 座標での単位ベクトル）で確認できる（:ref:`sec-body-basis`）。
   CAD ソフトの多くは慣性乗積を :math:`+\int xy\,\mathrm{d}m` の符号で表示する。IgnisYeet に与えるのはテンソルの要素 :math:`-\int xy\,\mathrm{d}m` なので、
   CAD の値の **符号を反転して** ``ixy_dry`` などに入れる。慣性テンソルが正定値でなければエラーになる。

.. _sec-usage-inertia-estimate:

.. note::

   **CAD がないときの慣性モーメントの見積もり。** 乾燥質量 :math:`m`、全長 :math:`L`、外半径 :math:`R` の機体を、一様な円柱とみなすと、重心まわりの値は
   :math:`I_{xx}=\tfrac12 mR^2`\ （ロール）、:math:`I_{yy}=I_{zz}=m\,(3R^2+L^2)/12`\ （ピッチ・ヨー）である。
   機体が薄肉の円筒に近ければ、:math:`I_{xx}=mR^2`、:math:`I_{yy}=I_{zz}=m\,(6R^2+L^2)/12` を使う。
   質量が一様でない機体は、部品（ノーズ、胴体、フィン、モータケース、電装など）に分け、部品ごとの質量 :math:`m_i` と乾燥重心からの軸方向の距離 :math:`d_i` から
   :math:`I_{yy}=\sum_i\bigl(I_{yy,i}+m_id_i^2\bigr)` と足し合わせる（平行軸の定理）。ロールは :math:`d_i` の項が入らず、:math:`I_{xx}=\sum_iI_{xx,i}` である。
   質量の分布が分からないときの見積もりは粗く、ピッチ慣性は質量の偏りで大きく変わる。モンテカルロ解析の ``dry_mass`` で幅を見るほか、値を数割変えた飛翔で結果の変化を確かめるとよい。
   モンテカルロ解析は、慣性を単独ではばらつかせず、``dry_mass`` のばらつきの倍率を乾燥質量と乾燥慣性テンソルのすべての要素に同じ倍率で掛ける。
   乾燥重心のばらつき（``cg``）は慣性モーメントを変えない。

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
   * - ``misalignment_deg``
     - ``0``
     - 推力の方向と機軸のなす角 :math:`\varepsilon` [deg]（:ref:`sec-thrust-line`）
   * - ``misalignment_azimuth_deg``
     - ``0``
     - 推力の傾きの向き :math:`\phi` [deg]。:math:`+y_b` から :math:`+z_b` へ測る
   * - ``misalignment_offset``
     - ``[0, 0]``
     - ノズル出口の点 ``[y, z]`` [m]（機体座標）。推力線はこの点を通る

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
     - 機体が発射位置からレールに沿って進み、レールを離れるまでの距離 [m]（有効な案内長）。後端のラグがレールの端にないときは、レールの物理的な長さではなく、ラグが発射位置からレールの端まで動ける距離を入れる（:ref:`sec-rail`）
   * - ``elevation_deg``
     - 必須
     - 射角（水平面からの仰角）[deg]
   * - ``azimuth_deg``
     - 必須
     - 射方位（北から時計回り）[deg]
   * - ``roll_deg``
     - ``0``
     - ランチャ上の機体のロール角 [deg]。機軸まわりに回す。0 では :math:`+y_b` が水平（:ref:`sec-initial-attitude`）

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
   * - ``main_cd_s``
     - なし
     - 二段開傘のメインの抗力面積 :math:`C_D S` [m\ :sup:`2`]（正）。``main_altitude`` と **同時に** 指定するか、両方とも省略する（片方だけはエラー）。
       指定すると、``cd_s`` がドローグになり、メインの開傘高度で抗力面積が ``main_cd_s`` に **置き換わる**\ （足し合わせない。:ref:`sec-dual-deploy`）。
       モンテカルロ解析の ``parachute_cd_s_scale`` は ``cd_s`` と ``main_cd_s`` の両方に掛かる
   * - ``main_altitude``
     - なし
     - メインの開傘高度 [m]。射点からの高さ（AGL）で、下降中にこの高さを横切った時刻に開傘する（正）。
       ドローグが開傘する時にすでにこの高さ以下なら、ドローグの開傘と同時にメインが開く

二段開傘の例（ドローグ 0.5 m\ :sup:`2`、300 m でメイン 8 m\ :sup:`2`）。

.. code-block:: toml

   [recovery]
   enabled = true
   cd_s = 0.5
   delay = 1.0
   main_cd_s = 8.0
   main_altitude = 300.0

.. _sec-config-aero:

``[aero]``\ （手法の選び方は :ref:`sec-aero-select`）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``method``
     - ``"barrowman"``
     - 空力係数の求め方。``"barrowman"`` は STL から抽出した形状に対する部品積み上げ法（:doc:`barrowman`）、
       ``"table"`` は外部の係数表の読み込み、``"panel"`` はパネル法（:doc:`panel`）、``"cfd"`` は SU2 による CFD（:doc:`cfd`）
   * - ``table``
     - なし
     - ``method = "table"`` のとき必須。係数表の CSV ファイル（:ref:`sec-usage-external-table`、形式は :ref:`sec-aero-table-format`）
   * - ``mach_min`` / ``mach_max`` / ``mach_step``
     - ``0`` / ``3`` / ``0.02``
     - 係数表の Mach 数の範囲と刻み。``mach_max`` は 10 以下
   * - ``alpha_max_deg`` / ``alpha_step_deg``
     - ``30`` / ``1``
     - 係数表の迎角の範囲（0 から。付属のサンプルは 90）と刻み [deg]。``alpha_max_deg`` は 180 以下。格子点の数（Mach 数の数 × 迎角の数）は 10\ :sup:`6` 以下
   * - ``n_slices``
     - ``600``
     - 形状抽出で STL を切る断面の数（20 以上）
   * - ``fin_threshold``
     - ``0.05``
     - フィン判定のしきい値（最大胴体半径に対する比）
   * - ``extrapolation``
     - ``"linear"``
     - 迎角と横滑り角が表の範囲外のときの扱い: ``"linear"`` （線形外挿。符号と範囲を制限する）または ``"clamp"`` （端の値で一定）。Mach 数は、この値にかかわらず端の値で一定（:ref:`sec-aero-interp`）
   * - ``table_kind``
     - ``"axisymmetric"``
     - 飛翔計算に使う係数表の種類。``"axisymmetric"`` （全迎角の軸対称表）または ``"general"`` （迎角と横滑り角の 6 成分の一般表。:ref:`sec-aero-table`）
   * - ``general_mach_step``
     - ``0.05``
     - ``table_kind = "general"`` で ``method = "barrowman"`` のときの Mach 数の刻み（正。:ref:`sec-aero-table-format`）

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

各項目の意味は :doc:`panel`、``panel_cp.csv`` の列は :ref:`sec-file-panel-cp` で説明する。

``[aero.cfd]``\ （``method = "cfd"`` のときだけ使う。SU2、gmsh、Open MPI が必要で、``surface = "stl"`` では manifold3d も要る。使い方は :doc:`cfd`、内部は :doc:`cfd_internals` で説明する）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"euler"``
     - 支配方程式。``"euler"`` は非粘性の Euler 方程式（粘性の抗力項は部品積み上げ法で補う）、``"rans"`` は Spalart–Allmaras モデルの RANS（信頼できない。結果に頼らない。:ref:`sec-cfd-run`）
   * - ``machs``
     - ``[0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0]``
     - 解く Mach 数（正、昇順、重複なし）
   * - ``alphas_deg``
     - ``[0, 2, 4, 8, 12, 16]``
     - 解く迎角 [deg]（0 以上、昇順、0 を含み、0 より大きい値が 1 つ以上）
   * - ``alpha_mode``
     - ``"mirror"``
     - 迎角の正負と :math:`\alpha=0` のずれの扱い。``"mirror"`` は正負の解の奇関数・偶関数部分を使う（:math:`z` 対称メッシュでは負の迎角は解かない）、
       ``"offset"`` は :math:`\alpha=0` の値を引く、``"single"`` は結果をそのまま使う
   * - ``z_mirror_mesh``
     - ``true``
     - 1/4 領域をメッシュにして :math:`z=0` で鏡映し、:math:`z` について厳密に対称なメッシュにするか。形状が :math:`z` について対称でないときは無視する（:ref:`sec-cfd-stl-symmetry`）
   * - ``symmetry``
     - ``true``
     - ピッチ面（:math:`y=0`）で切った半モデルを使ってよいか。使う条件と領域の意味は :ref:`sec-cfd-stl-symmetry`。条件を満たさないときと ``false`` は全領域を解く
   * - ``surface``
     - ``"panel_mesh"``
     - 壁面の形状。``"panel_mesh"``\ （別名 ``"cad"``）は抽出した胴体とフィンを再構成する。``"stl"`` は入力の STL から壁面を作る（入力の形状と同一ではない。:ref:`sec-cfd-stl`、:ref:`sec-cfd-fidelity`）
   * - ``betas_deg``
     - ``[0]``
     - 解く横滑り角 [deg]（0 以上、昇順、0 で始まる）。0 だけでないと係数表は 6 成分の一般表になる（:ref:`sec-cfd-table-kind`）
   * - ``shape_symmetry``
     - ``"auto"``
     - 鏡映像として得て解かない対称性。``"axisymmetric"``\ （:math:`y=0` と :math:`z=0`）、``"pitch_plane"``\ （:math:`y=0`）、``"none"``\ （使わない）、``"auto"``\ （形状から判定する）。
       判定と、``stl`` で明示した対称性が検査に通らないときのエラーは :ref:`sec-cfd-stl-symmetry`、表の種類との関係は :ref:`sec-cfd-table-kind` で述べる
   * - ``roll_damping``
     - ``false``
     - 各 Mach 数でロール減衰の 2 ケースを回転座標系で解き、:math:`C_{lp}` を CFD の値にする（:ref:`sec-cfd-run`、:ref:`sec-cfd-roll-cases`）。Euler だけ
   * - ``roll_rate_hat``
     - ``0.01``
     - ロール減衰のケースの無次元ロール角速度 :math:`\hat p=pd/(2V)`\ （0 より大きく 0.05 以下）
   * - ``base_treatment``
     - なし
     - ``surface = "stl"`` かつ Euler のとき **必須**\ 。``"fairing"`` は底面の後ろに円すいを足す（入力の STL を変更する）、``"flat"`` は平らな底面のままにする（Euler の解は収束しないことがあり、収束しないケースは失敗。:ref:`sec-cfd-stl-base`）
   * - ``stl_remesh``
     - ``"classify"``
     - ``surface = "stl"`` の壁面の処理。``"classify"`` は gmsh で曲面を作り ``wall_size`` で作り直す、``"none"`` は STL の三角形をそのまま使う（フィンの解像が足りず法線力が低くなる）
   * - ``max_geometry_error``
     - ``wall_size``
     - ``surface = "stl"`` で、CFD の壁面が入力の STL または和から離れてよい最大の距離 [m]。超えるとエラー、``wall_size`` の半分を超えると警告（:ref:`sec-cfd-fidelity`）
   * - ``general_mach_step``
     - ``0.1``
     - 一般表の Mach 数の刻み。0 なら ``[aero]`` の ``mach_step``\ （格子は :ref:`sec-aero-table-format`）
   * - ``fin_roll_deg``
     - ``0``
     - フィンの組を機軸のまわりに回す角度 [deg]（0 でフィン 2 枚が :math:`y=0` 面に載る、45 で載らない）
   * - ``tail_fairing``
     - ``6.0``
     - Euler のみ。平らな底面の後ろに置く円すい形の尾部フェアリングの長さ（底面半径の倍数）。0 なら平らな底面。RANS は常に平らな底面
   * - ``farfield``
     - ``20.0``
     - 遠方境界の半径（全長の倍数、5 以上）。半球と球の半径、または箱の半辺
   * - ``wall_size``
     - ``0.002``
     - 壁面の要素の大きさ [m]。小さいほど細かく、セル数と時間が増える
   * - ``yplus``
     - ``1.0``
     - RANS の目標 :math:`y^+`
   * - ``iterations``
     - ``3000``
     - 1 ケースの最大反復数（RANS は 300 で打ち切る）
   * - ``min_iterations``
     - ``100``
     - 早期終了の最小反復数。これより前にはケースを止めない。``stable_window`` 以上（:ref:`sec-cfd-int-convergence`。Euler の Roe の制限関数もこの反復で固定する）
   * - ``stable_window``
     - ``60``
     - 早期終了の判定で、力とモーメントの係数が落ち着いているかを見る直近の反復数（10 以上。:ref:`sec-cfd-int-convergence`）
   * - ``cfl``
     - ``5.0``
     - CFL 数の初期値（適応的に変える）
   * - ``convergence``
     - ``1e-6``
     - 密度の二乗平均残差の目標（0 と 1 の間）
   * - ``scheme``
     - ``"roe"``
     - 対流フラックス。``"roe"`` または ``"jst"``\ （:ref:`sec-cfd-int-discretization`）
   * - ``multigrid``
     - ``0``
     - 凝集型マルチグリッドの粗い格子の数（``MGLEVEL``、0 から 4）。壁面の三角形が多い四面体メッシュでは効果がない（:ref:`測定 <sec-cfd-multigrid>`）
   * - ``multigrid_cycle``
     - ``"w"``
     - マルチグリッドのサイクル。``"w"`` または ``"v"``
   * - ``prefix``
     - ``""``
     - CFD ツールの環境（``<prefix>/bin/SU2_CFD``、``mpirun``、gmsh を持つ ``python``）。空なら環境変数 ``IGNISYEET_CFD_PREFIX``、それも空なら ``PATH``。先頭の ``~`` は展開する
   * - ``su2`` / ``mpi``
     - ``"SU2_CFD"`` / ``"mpirun"``
     - SU2 の実行ファイル名（またはパス）と MPI の起動コマンド（``ranks_per_case`` が 2 以上のときだけ使う）
   * - ``ranks_per_case``
     - ``4``
     - 1 ケースの MPI ランク数。0 で自動（4 とスレッド数の小さいほう）
   * - ``parallel_cases``
     - ``4``
     - 同時に解くケース数。0 で自動（メモリとスレッドの予算に収まる数）。``ranks_per_case`` × ``parallel_cases`` が使えるスレッド数を超えると警告する
   * - ``timeout_minutes``
     - ``0``
     - 1 ケースの時間の上限 [分]。0 で無制限。上限に達したケースは、係数が落ち着いていれば受け入れ、そうでなければ失敗とする
   * - ``workdir``
     - ``"cfd"``
     - ``output.dir`` の下の作業ディレクトリ（ケースごとのサブディレクトリとメッシュのキャッシュ）

CFD の準備は ``ignisyeet cfd-check 設定ファイル [--mesh]`` で確かめる（:doc:`cfd`）。``surface = "stl"`` の ``--mesh`` は、形状の忠実度も表示する。
結果は ``cfd_cases.csv`` と ``cfd_report.json`` に書かれる（後述）。

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
     - 標準大気の温度に一律に足す温度差 :math:`\Delta T` [K]（``us1976`` のみ。:math:`-186.946` K より大きい値）
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
     - 基準高度での風速 [m/s]（0 以上）。``profile`` では使わず、表の平均風速（:ref:`sec-wind-profile`）で置き換える
   * - ``direction_deg``
     - ``0.0``
     - 風が **吹いてくる** 方位（北から時計回り）[deg]。``profile`` では使わず、表の平均風の風向で置き換える
   * - ``ref_height``
     - ``2.0``
     - 風速の基準高度 [m]（正）
   * - ``exponent``
     - ``6.0``
     - べき法則の指数 :math:`n`\ （正。``power`` のみ）
   * - ``roughness_length``
     - ``0.03``
     - 粗度長 :math:`z_0` [m]（``log`` のみ。0 より大きく ``ref_height`` の半分より小さい値）
   * - ``ground_blend_height``
     - ``1.0``
     - 地面付近で速度を 0 になめらかにつなぐ高さ [m]（``power`` と ``log`` のみ。正で ``ref_height`` より小さい値）。``log`` では :math:`z_0` より大きい必要があり、設定値が :math:`z_0` 以下なら :math:`\max(\text{値}, 2 z_0)` に読み替える。地面での無限大・不連続な勾配を取り除き、適応刻み幅の積分が着地付近で極端に小さくならないようにする
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
     - 数値積分法。``"rk4"``\ （固定刻みの古典的 Runge–Kutta 法）、``"rk45"``\ （刻み幅制御付きの Dormand–Prince 5(4) 法）、
       ``"dop853"``\ （刻み幅制御付きの Dormand–Prince 8(5,3) 法）のいずれか（:ref:`sec-integrators`）
   * - ``attitude``
     - ``"normalize"``
     - 姿勢の更新法。``"normalize"`` は四元数を通常の状態量として積分し、各ステップ後に正規化する。
       ``"lie_group"`` は SO(3) 上の Runge–Kutta–Munthe-Kaas 法で、正規化をせずに四元数のノルムが丸め誤差の範囲で 1 に保たれる。
       どの積分法とも組み合わせられる（:ref:`sec-integrators`）
   * - ``dt``
     - ``0.002``
     - ``rk4`` の時間刻み [s]。``rk45``・``dop853`` では最初の刻み幅として使う（0.05 s 以下）
   * - ``rtol`` / ``atol``
     - ``1e-7`` / ``1e-6``
     - ``rk45``・``dop853`` の相対許容誤差・絶対許容誤差（いずれも正で 10\ :sup:`-3` 以下）。ステップは ``output_interval`` でも上限が抑えられる（燃焼中とレール上は 0.05 s 以下）ので、通常の飛翔では許容誤差を小さくしても結果がほとんど変わらないことがある
   * - ``max_time``
     - ``3600``
     - 計算を打ち切る時刻 [s]（正）。この時刻までに着地しなかった飛翔は着地に失敗したものとして扱い、``sim`` は警告して終了ステータス 3 で終わる。落下分散では、そのケースとサンプルを失敗に数える（:doc:`dispersion`）
   * - ``output_interval``
     - ``0.05``
     - ``trajectory.csv`` に書き出す間隔 [s]。``rk45``・``dop853`` では刻み幅の上限でもある（正で ``max_time`` 以下）
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
     - ``wind_grid`` で計算する風速の一覧 [m/s]（0 以上、100 個以下）。``constant`` ``power`` ``log`` では基準高度 ``wind.ref_height`` の風速、``profile`` では表の平均風速（:ref:`sec-wind-scaling`）。選び方は :ref:`sec-dispersion-safety`
   * - ``directions``
     - ``8``
     - ``wind_grid`` の風向の分割数（北から等間隔。1 以上 360 以下）

``[dispersion.monte_carlo]``\ （``mode = "monte_carlo"`` のときに使う）

各ばらつきは互いに独立な正規分布 :math:`N(0,\sigma^2)` から引き、以下の値は標準偏差 :math:`\sigma` である。
0 を指定するとそのばらつきを無効にする。負の値や有限でない値はエラーになる。
「相対」は公称値に対する割合（0.03 なら 3 %。公称値に :math:`1+\text{乱数}` を掛け、下限は 0.05）、
「絶対」は公称値に加える量（単位は項目ごとに示す）である。

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

   * - 項目
     - 既定値
     - 意味
   * - ``samples``
     - ``1000``
     - サンプル数（1 以上 10\ :sup:`6` 以下）
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
     - 絶対 [deg]。レールの向きの設置誤差 :math:`\sigma_{\mathrm{tilt}}`。公称の軸を、面内（射角）と面に垂直な 2 方向の独立な傾きで等方に傾ける。傾けた射角が :math:`(0,90]` の外のサンプルは失敗になる（:ref:`sec-mc-rail`）
   * - ``azimuth_deg``
     - ``1.0``
     - 絶対 [deg]。射方位の設定誤差。傾けたレールを鉛直軸の周りに回す（水平方向の効果は :math:`\sigma\cos(\text{射角})`）
   * - ``wind_speed``
     - ``1.0``
     - 絶対 [m/s]。基準風速（``[wind]`` の ``speed`` の周りに分布し、0 未満は 0 にする）。``profile`` では表の平均風速 :math:`W` の周りの揺らぎで、表を :math:`\max(W+\delta,0)/W` 倍する（:ref:`sec-wind-scaling`）。表がほぼ無風（平均 0.1 m/s 未満）で、この値が 0 でないときはエラー
   * - ``wind_direction_deg``
     - ``15.0``
     - 絶対 [deg]。風向（``[wind]`` の ``direction_deg`` の周りに分布する。``profile`` では表の平均風の風向の周り）
   * - ``parachute_cd_s_scale``
     - ``0.1``
     - 相対。パラシュートの抗力面積 :math:`C_DS`
   * - ``thrust_misalignment_deg``
     - ``0``
     - 絶対 [deg]。推力の傾き。:math:`y_b`, :math:`z_b` 方向の傾き成分に独立な正規乱数を足し、傾きの角度と方位を決める
   * - ``thrust_offset``
     - ``0``
     - 絶対 [m]。ノズル出口の点。:math:`y_b`, :math:`z_b` 方向に独立な正規乱数を足す
   * - ``fin_cant_deg``
     - ``0``
     - 絶対 [deg]。設定したフィンのカントに加える量
   * - ``cg_lateral``
     - ``0``
     - 絶対 [m]。乾燥重心の横ずれ。:math:`y_b`, :math:`z_b` 方向に独立な正規乱数を足す
   * - ``cl_scale``
     - ``0``
     - 相対。ロールのモーメント係数 :math:`C_l` とカントの傾き :math:`C_{l\delta}`
   * - ``clp_scale``
     - ``0``
     - 相対。ロール減衰微係数 :math:`C_{lp}`

最後の 6 項目（``thrust_misalignment_deg`` から ``clp_scale``）は既定で無効である。
乱数はこれらを表の最後に引くので、最初の 11 項目の値はこの 6 項目の設定に依存しない。
各項目の扱いの詳細は :ref:`sec-mc` で述べる。

.. _sec-usage-external-table:

外部の係数表を使う（``aero.method = "table"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

風洞試験や CFD などで得た係数を使うときは、設定に次の 2 項目を書く。

.. code-block:: toml

   [aero]
   method = "table"
   table  = "cfd_table.csv"

STL からの形状抽出と空力モデルの計算は行わず、``sim`` と ``dispersion`` はこの表をそのまま使う（``aero`` は表の行数を表示するだけである）。
表は、``out/aero_table.csv`` と同じ形式の CSV と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つで用意する。
列、付随ファイルの項目、一般表の格子、ロール項の補い方は :ref:`sec-aero-table-format` にある。
表の考え方は :ref:`sec-aero-external` を参照されたい。

出力ファイル
------------

出力先ディレクトリ（``[output] dir``）に書かれる主なファイルを示す。列と項目の仕様は :doc:`file_formats` にまとめてある。

.. list-table::
   :header-rows: 1
   :widths: 30 70
   :class: longtable

   * - ファイル
     - 内容
   * - ``geometry.json``
     - 抽出した形状（全長、基準直径、ノーズ、ボートテール、フィン、断面ごとの半径など）と、機体軸の STL 座標での単位ベクトル ``body_x``, ``body_y``, ``body_z``\ （:ref:`sec-body-basis`、:ref:`sec-file-geometry`）
   * - ``profile.csv``
     - 断面ごとの胴体半径 ``r`` とフィン高さ ``fin_span``
   * - ``aero_table.csv`` / ``.json``
     - 空力係数表と、その種類・格子・基準量・入力のハッシュ値の付随ファイル（:ref:`sec-aero-table-format`）
   * - ``aero_drag.csv``
     - Mach 数ごとの抗力の内訳、:math:`C_{N\alpha}`、:math:`x_{cp}`\ （``method = "barrowman"`` と ``"panel"``。:ref:`sec-file-drag`）
   * - ``cfd_cases.csv``
     - ``method = "cfd"``。ケースごとの状態、反復数、残差、SU2 の生の係数、表に使う係数、計算時間（:ref:`sec-file-cfd`）
   * - ``cfd_report.json``
     - ``method = "cfd"``。ケース数、失敗と未収束の一覧、メッシュの大きさ、計算時間、表の種類、CFD の値を使う範囲、ロール減衰の採用元、STL の対称性と忠実度の判定など（:ref:`sec-file-cfd`）
   * - ``<workdir>/mesh/``
     - ``method = "cfd"`` のメッシュのキャッシュ。``surface = "stl"`` では修復と和の報告 ``repair_report.json`` ``union_report.json`` と、形状の忠実度 ``stl_fidelity.json`` も入る（:ref:`sec-file-mesh`、:ref:`sec-cfd-fidelity`）
   * - ``panel_cp.csv``
     - ``method = "panel"``。表面パネルの位置・法線・面積・圧力係数 ``cp`` と部位。``python/plot.py panel`` で作図する（:ref:`sec-file-panel-cp`）
   * - ``trajectory.csv``
     - 飛翔の時刻歴（位置、速度、Mach 数、迎角、質量、重心、圧力中心、安定余裕、姿勢、角速度など。:ref:`sec-file-trajectory`）
   * - ``summary.json``
     - ランチャ離脱速度、最大 Mach 数、最大動圧、頂点、最小安定余裕、着地点など。着地の有無 ``landed``、``max_time``、設定のハッシュ ``config_hash``、警告 ``warnings``、係数表の範囲外の引き出しの集計 ``aero_table_range`` も入る。``sim`` が失敗したときは、失敗の印 ``{"failed": true, "error": "…"}`` だけになる（:ref:`sec-file-summary`）
   * - ``ignisyeet.kml``
     - ``kml = true`` のとき ``sim`` と ``dispersion`` が書き出す、Google Earth 用の KML ファイル 1 つ。射点、飛翔経路（段階ごとに色分け）とイベントの印、風の格子の着地点の輪郭、モンテカルロの着地点と誤差楕円をフォルダに分けて収める（:ref:`sec-kml`）
   * - ``dispersion.csv``
     - ``mode = "wind_grid"``。風の各ケースの着地点、頂点、飛行時間など。着地したかを示す ``landed`` の列と、静的に安定かを示す最後の列 ``stable`` もある
   * - ``dispersion_meta.json``
     - ``mode = "wind_grid"``。``dispersion.csv`` に付く、設定のハッシュ ``config_hash``、ケース数 ``cases``、失敗数 ``failed``、降下モードごとの不安定なケースの数 ``unstable``、警告 ``warnings``、``aero_table_range``\ （:ref:`sec-file-dispersion`）
   * - ``dispersion_mc.csv``
     - ``mode = "monte_carlo"``。サンプルごと・降下モードごとの 1 行（ばらつき、基準風、着地点、頂点など。失敗した行は結果の列が空欄になる）。最後の列 ``stable`` が静的に安定かを示す
   * - ``dispersion_summary.json``
     - ``mode = "monte_carlo"``。降下モードごとの着地点の統計と誤差楕円、最大着地距離、着地距離のパーセンタイル、頂点高度の平均と標準偏差、統計が完全か（``complete``）、降下モードごとの不安定なサンプルの数 ``unstable``、設定のハッシュ、警告、``aero_table_range``

.. _sec-kml:

Google Earth で見る（KML）
--------------------------

``[output]`` の ``kml`` が ``true``\ （既定）のとき、``sim`` と ``dispersion`` は ``ignisyeet.kml`` を、CSV や JSON と同じ出力先ディレクトリに書き出す。
KML は地理情報を記述する XML 形式で、OGC の標準（KML 2.2 :cite:`ogckml`）である。Google Earth（デスクトップ版・Web 版）や QGIS などで開くと、機体の経路や着地点を衛星写真の上に重ねて確認できる。
不要なときは ``kml = false`` にすればよい。各要素をクリックすると、時刻・速度・着地点などの数値が説明欄に表示される（説明欄の文は英語である）。
説明欄の文は CDATA に入れ、``<`` や ``&`` はそのまま書く（要素の名前は通常の XML の文字列として、エスケープして書く）。

``ignisyeet.kml`` は 1 つの KML 文書で、次のフォルダ（Google Earth の左のパネルでチェックボックスにより表示を切り替えられる）からなる。
``sim`` と ``dispersion`` は別のコマンドなので、どちらも実行のたびにこのファイルを書き直す。
そのコマンドで計算した結果に加え、同じ出力先ディレクトリにある他の結果（``trajectory.csv`` と ``summary.json``、``dispersion.csv``、``dispersion_mc.csv`` と ``dispersion_summary.json``）を読み戻して、1 つにまとめる。
該当するファイルがなければ、そのフォルダは作らない（読めないファイルがあるときは、そのフォルダを省いたことを端末に表示する）。
したがって、``sim`` のあとに ``dispersion`` を実行すれば、飛翔経路と落下分散を同じ画面で重ねて見られる。
読み戻す結果は、同じディレクトリに書いた、前の実行のものである。
読み戻すのは、設定のハッシュ（``config_hash``）が現在の設定と一致するものだけである。ハッシュは出力先と計算資源（``[output]`` と ``[resources]``）を除く設定と、STL、モータ、風、外部の係数表の内容から計算する。
``summary.json`` と ``dispersion_meta.json``、``dispersion_summary.json`` にあるハッシュが現在の設定と違う（またはハッシュがない）ときは、その結果は古いものとして KML に入れず、警告を表示する。
ハッシュは、飛翔は ``dispersion`` の設定を除き、風の格子は ``mode`` とモンテカルロの設定を除き、モンテカルロは ``mode`` と風の格子の設定を除いて計算する。
したがって、設定を変えたときは、``sim`` と ``dispersion`` の両方を実行し直せば、新しい結果だけが入る。
着地しなかった飛翔と、着地しなかった風のケースは KML に入れない（風の格子で着地したケースが一つもなければ、そのフォルダは作らない）。モンテカルロでは、着地したサンプルだけを入れる。

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
   モンテカルロの結果（``dispersion_mc.csv`` と ``dispersion_summary.json``）があるときに入る。降下モードごとのフォルダに、平均着地点（Mean landing）、:math:`1\sigma` と :math:`3\sigma` の誤差楕円（:eq:`eq-mc-stat` の共分散から作る 72 頂点の閉じた折れ線で、塗りつぶさない）、
   各サンプルの着地点（``Landing points`` フォルダ）が入る。失敗したサンプルと着地しなかったサンプルは含まない。

落下分散の図形はすべて地面に固定して描く。

**高度の扱い。** 飛翔経路の高度は ``absolute``\ （絶対高度）で書き出す。値は WGS84 楕円体からの高さで、射点で ``launch.altitude`` から始まり、
飛翔中は ``launch.altitude`` に射点からの上向きの高さを足したものになる（ECEF モードでは楕円体高そのもの）。
KML の絶対高度は名目上は平均海面（EGM96 ジオイド）からの高度だが、IgnisYeet はジオイド高（楕円体と平均海面の差）を **扱わない**。
場所によっては数十 m の差になるので、``launch.altitude`` には射点の **海抜標高** を入れ、地形の高さと合っていることを確かめてほしい。
合っていなければ、経路が地面にめり込んだり浮いたりして見える。射点・着地点・誤差楕円など地面に固定する図形は、Google Earth の地形に沿って表示される。

ディレクトリ構成
----------------

リポジトリの主なファイルとディレクトリを :numref:`fig-directory-tree` に示す。

.. _fig-directory-tree:

.. figure:: _generated/tikz/directory_tree.*
   :width: 100%

   リポジトリの構成。フォルダのアイコンはディレクトリ、紙のアイコンはファイルを表す。

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

.. _sec-development:

開発者向け：CI とリリース
--------------------------

この節は、IgnisYeet 自体を開発・配布する人向けである。利用だけなら読まなくてよい。

リリースとバージョン
~~~~~~~~~~~~~~~~~~~~


IgnisYeet のバージョンは ``Cargo.toml`` の ``[workspace.package] version``\ （``ignisyeet --version`` で表示される値）で決まり、
GitHub では同じ番号のタグ ``vX.Y.Z`` と GitHub Release で管理する。インストーラの user は、
``--version`` を指定しなければ最新のリリースを入れる。

push とプルリクエストでは、ワークフロー ``ci.yml`` が ``cargo build --release --locked`` と ``cargo test --release --locked`` を実行し、
例題で ``geom``、``aero``、``sim``、風の格子の ``dispersion``、モンテカルロの ``dispersion``、``python/plot.py`` を通しで動かし、
``python/plot.py`` の単体テスト（``python/test_plot.py``）も実行し、
``sim.max_time`` を短くした飛翔が終了ステータス 3 で終わることも確かめる（clippy と rustfmt は ``-D warnings`` と ``--check`` で実行し、違反があるとジョブが失敗する。ShellCheck は助言のみで、失敗してもジョブは落ちない）。
別のジョブが、必要な最低の Rust の版 1.87 で ``cargo build --locked`` を実行する。
ワークフローが使う Action は、コミットの SHA で固定している。
書式は ``rustfmt.toml``\ （``max_width = 160``）で定める。Rust のツールチェーンは ``rust-toolchain.toml``\ （stable、rustfmt と clippy を含む）で指定し、``Cargo.toml`` の ``rust-version``\ （1.87）が必要な最低の版を定める。

タグ ``v*`` を push すると、GitHub Actions のワークフロー ``release.yml`` が次の処理を行う。

#. タグと ``Cargo.toml`` のバージョンが一致することを確かめる（一致しなければ失敗する）。
#. 4 つのターゲット向けにバイナリを作る。Linux は musl で静的にリンクするので、ディストリビューションによらず動く。

   .. list-table::
      :header-rows: 1
      :widths: 40 30 30

      * - ターゲット
        - ビルド環境
        - テスト
      * - ``x86_64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行する
      * - ``aarch64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行しない（クロスビルド）
      * - ``x86_64-apple-darwin``
        - macOS（Intel）
        - 実行する
      * - ``aarch64-apple-darwin``
        - macOS（Apple Silicon）
        - 実行する

#. ``scripts/package-release.sh`` で ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と SHA-256 を作る。
   中身はバイナリ（``bin/ignisyeet``）、例題（Git で管理しているファイルだけ。``examples/out*`` や CFD の作業ディレクトリなど、Git の管理外の出力は入らない）、作図スクリプト（``python/plot.py`` と、依存関係の一覧および固定した版 ``uv.lock`` ``plot-constraints.txt``）、
   ``LICENSE``、``NOTICE``、``CITATION.cff``、``README.md`` である。
   アーカイブは GNU tar で、ファイルの順序、時刻（``SOURCE_DATE_EPOCH``、既定は 0）、所有者を固定して作る。macOS のビルドでは ``gnu-tar`` を入れる。
#. GitHub Release を作り、すべての配布物、チェックサム、``install.sh`` を添付する。
   リリースノートは前のタグからのコミットを Conventional Commits の種類（feat、fix、docs など）ごとにまとめて作る。

新しいバージョンを出すときは、``Cargo.toml``\ （と ``CITATION.cff`` などの表記）のバージョンを上げてコミットし、
同じ番号のタグを付けて push する。

.. code-block:: sh

   git tag v0.4.0
   git push origin v0.4.0
