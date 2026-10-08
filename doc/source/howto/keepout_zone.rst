.. _page-howto-keepout-zone:
.. _sec-dispersion-safety:

保安距離を求める
==================

落下分散の結果を保安距離（保安域の半径）の根拠にするときの手順を示す。
1 回の ``ignisyeet dispersion`` の最大着地距離は、計算した風速・風向・サンプルの中での最大値にすぎない。計算していない条件の着地点は、これより遠くなりうる。
そこで、遠い着地点を取りこぼさないように条件の組合せを広げ、すべての最大値の最大に余裕を足して半径とする。

このページを終えると、bash のコマンドで次の計算を順に行い、半径を決められる。

* 打上げ可能な風速の上限までの風の格子を、最大値が変わらなくなるまで細かくする。
* 格子で最も遠くなった風で、機体のばらつきのモンテカルロ解析を行う。予報の風でのモンテカルロ解析も行う。
* 二段開傘の機体では、正常な開傘に加えて、メインが頂点で開く場合とドローグだけで降りる場合を計算する。
* 以上を部品積み上げ法とパネル法の両方で計算する。

手順の全体を :numref:`fig-keepout-flow` に示す。
数値の基準（保安距離そのものの規定）は、打上げを行う場所と団体の規則に従う。ここでは規定の数値は与えない。
この手順がなぜ必要か、サンプル数と最大値の関係、条件を重ねる範囲の考え方は :ref:`page-theory-dispersion` にある。
結果を使う前に :ref:`page-theory-limitations` も読む。

.. _fig-keepout-flow:

.. figure:: /_generated/tikz/keepout_flow.*
   :width: 100%

   保安距離を求める手順 0 から 7 の流れ。上段の青い箱は公称の機体の計算と風の探索、下段の青い箱は機体のばらつきと空力の手法の違いを含める計算、橙の箱は二段開傘の機体だけの計算である。すべての計算の最大値の最大に余裕を足して半径とする。

前提
------

* ``ignisyeet sim`` が設定ファイルで正常に終わる（:ref:`page-howto-read-results`）。
* 風の設定が決まっている。保安の検討には高度別の表（``wind.model = "profile"``）を使う。``power`` と ``log`` は地上の基準風速を上空へ外挿するので、上空の風が強い日の着地点を過小にも過大にも見積もる（:ref:`page-theory-wind`）。
  例の ``examples/wind_profile_example.csv`` は書式を示す架空の表なので、実際には当日の予報や観測の表に置き換える（:ref:`page-howto-environment`）。
* 回収系の設定が済んでいる（:ref:`page-howto-launcher-parachute`）。
* 打上げを中止する風速（ランチ風速の上限）と、その風速を測る高さが決まっている。
* **bash** と ``python3`` が使える。zsh などを使っているときは、先に ``bash`` を起動する。

計算時間の目安：例の高度別の表で手順 0 から 4 のすべてを ``rk45`` で計算すると、20 スレッドの CPU で約 8 分かかる。``rk4`` のままだと 1 時間半ほどかかるので、下の ``P`` に ``--set sim.integrator=rk45`` を加える。4 コアのノート PC では数十分から 1 時間以上を見込む。
測定の条件は :ref:`page-vv-performance` にある。

手順
------

準備
~~~~

手順は、1 つの設定ファイルを書き換えずに、コマンドラインの ``--set``、``--out``、``--aero-table``\ （:ref:`page-howto-sweeps`）で条件を変えて実行する。
``--set`` の組は bash の配列に入れ、``"${P[@]}"`` の形で展開する。こうすると、``[7.8, 11.9]`` のように空白や角かっこを含む値も 1 つの引数のまま渡る。
結果は、現在のディレクトリの ``runs/`` の下に計算ごとに分けて書く。

まず ``ignisyeet`` を ``PATH`` に入れ、設定ファイルのパスを ``C`` に入れる。ソースからビルドしたときは、リポジトリの最上位のディレクトリで次を実行する。
インストーラで入れたときは ``~/.local/bin`` が ``PATH`` に入っていればよく（:ref:`page-howto-install`）、例題をコピーしたディレクトリの設定ファイルを ``C`` にする。
リポジトリの外で作業するときは、``PATH`` にリポジトリの ``target/release`` を絶対パスで入れる。
以下のコマンドは、``runs/`` を作りたいディレクトリに ``cd`` してから実行する。

.. code-block:: bash

   export PATH="$PWD/target/release:$PATH"   # ソースからビルドしたとき
   C=examples/sample.toml                    # インストーラなら
                                             # C=ignisyeet-examples/sample.toml

各コマンドの終了ステータスを残すため、``ignisyeet`` を関数 ``run`` で包む。終了ステータスが 0 でなかったコマンドは ``runs/status.txt`` に書かれる（手順 7 で確かめる）。

.. code-block:: bash

   mkdir -p runs && : > runs/status.txt
   run() {
     ignisyeet "$@" || echo "exit $?: ignisyeet $*" >> runs/status.txt
   }

格子で最も遠くなった風を読む関数 ``worst`` も定義する。引数は出力先と降下モード（``ballistic`` または ``parachute``）である。
その出力先の ``dispersion.csv`` で、その降下モードの着地したケースのうち ``landing_distance`` が最大の行の ``wind_speed`` と ``wind_direction_deg`` を出す。

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

高度別の風の表は ``wind.profile`` で与える。``profile`` では ``[wind]`` の ``speed`` と ``direction_deg`` は使われない。表の風を変えるときは、``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` で表全体を拡大・回転する（:ref:`sec-wind-scaling`）。
サンプルの設定ファイルは ``speed`` と ``direction_deg`` を書いているので、以下のコマンドでは、使わないことを知らせる警告（``warning[IY-C502]: wind.speed is ignored for wind.model = "profile"; …``）が毎回出る。結果には影響しない。自分の設定ファイルでは、``[wind]`` からこの 2 つを消しておく。

すべての計算に共通の ``--set`` を配列 ``P`` に、二段開傘の正常な開傘の 2 項目を配列 ``N`` に入れる。
``--set`` は項目を消せないので、二段開傘の機体では ``recovery.main_cd_s`` と ``recovery.main_altitude`` を設定ファイルに書かず（ファイルの ``recovery.cd_s`` はドローグの抗力面積）、手順 0 から 4 のすべての計算に ``"${N[@]}"`` で加える。
こうすると、手順 0 から 4 は正常に開傘する機体を飛ばし、手順 5 では ``N`` を除く（ドローグだけ）か、``N`` の代わりにメインが頂点で開く設定を入れて、故障を飛ばせる。単一のパラシュートの機体では ``N=()``\ （空の配列）とする。

.. code-block:: bash

   P=(--set wind.model=profile
      --set wind.profile=wind_profile_example.csv)
   N=()   # 単一のパラシュート

二段開傘の機体として試すとき（下の計算例。ドローグ 0.5 m\ :sup:`2`、高度 300 m でメイン 8 m\ :sup:`2`）は、続けて次を実行する。

.. code-block:: bash

   P+=(--set recovery.cd_s=0.5)   # ファイルの cd_s をドローグの値に
   N=(--set recovery.main_cd_s=8 --set recovery.main_altitude=300)

KML が要らない一括の計算では、``P`` に ``--set output.kml=false`` を加えると出力が小さくなる（結果は変わらない）。
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
     - 正常な開傘（``N``）。風は手順 2（正常な開傘）の格子で最も遠くなった風
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

手順 0　係数表を作り、表の平均風を確かめる
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

空力の手法ごとに係数表を一度だけ作り、以後の計算は ``--aero-table`` でそれを使う（手順 6 のため、部品積み上げ法とパネル法の両方を作る）。

.. code-block:: bash

   for m in barrowman panel; do
     run aero "$C" --set aero.method=$m --out runs/aero_$m
   done
   run sim "$C" "${P[@]}" "${N[@]}" \
       --aero-table runs/aero_barrowman --out runs/nominal

期待する結果： ``runs/nominal/summary.json`` に表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` が ``wind_speed`` と ``wind_direction_deg`` として出る（:ref:`sec-wind-profile`）。
``wind.ref_height`` での表の風（手順 1 で使う）は、同じ ``summary.json`` の ``wind_at_ref_height`` に出て、端末には次の行が出る。

.. code-block:: text

   profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)

例の表では :math:`W=11.89` m/s、:math:`\Psi=220.2^\circ` である（``examples/wind_profile_example.csv``）。
開傘時の対気速度と動圧（手順 5）も、この ``summary.json`` に出る。

手順 1　風の格子を、打上げ可能な風速の上限まで、36 方位以上で計算する
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``dispersion.wind_speeds`` は、打上げを中止する風速までを含め、``directions`` は 36 以上（10° 間隔以下）にする。
``wind_speeds`` の意味は風モデルで異なる。

* ``constant``、``power``、``log``: ``wind.ref_height``\ （既定 2 m）での風速である。団体が風速の上限を測る高さ（地上 2 m、10 m など）に合わせて ``ref_height`` を設定し、その高さでの上限までを含める。
* ``profile``: 表の **平均風速** :math:`W` である（地上の風速ではない）。表の形はそのままに、全体を :math:`k=v/W` 倍にする。
  団体の上限 :math:`V_{\mathrm{lim}}`\ （測る高さ :math:`h_{\mathrm{lim}}` での風速）を表に当てはめるには、表のその高さの風速 :math:`w(h_{\mathrm{lim}})` を求め、上限の風速に当たる格子の風速を :math:`v_{\max}=W\,V_{\mathrm{lim}}/w(h_{\mathrm{lim}})` とする。
  :math:`h_{\mathrm{lim}}` を ``wind.ref_height`` に合わせておけば、:math:`w(h_{\mathrm{lim}})` は手順 0 の ``wind_at_ref_height`` の ``speed`` である。
  ``wind.ref_height`` と違う高さで測るときは、表のその高さの風速を、前後の行の線形補間で読む。
  格子には :math:`v=W`\ （表そのまま、:math:`k=1`）と :math:`v_{\max}` までの値を入れる。
  :math:`k` が 3 を超える、または 1/3 を下回る風速があると警告が出る。表の形を保った拡大が現実的でないので、上限の風速を与えた着地点の上空の風速を確かめ（``wind_scale`` と警告に出る上空の風速）、ふさわしくなければ、上限の風速ごとに別の表を用意する（:ref:`sec-wind-scaling`）。

例として、上限を地上 2 m（既定の ``wind.ref_height``）で 7 m/s とする。手順 0 の出力から :math:`w(2\ \mathrm{m})=3.04` m/s、:math:`v_{\max}=11.89\times7/3.04=27.4` m/s である。
格子には、地上 2 m で 2、4、5、6、7 m/s に当たる 7.8、15.6、19.6、23.5、27.4 m/s と、:math:`W` そのものの 11.9 m/s を入れる（:math:`k=0.66` から 2.3）。
格子の風速は配列 ``G`` に入れ、手順 2 と 5 でも使う。格子の最大の風速（上限の風速）は ``VMAX`` に入れ、手順 3 で使う。

.. code-block:: bash

   G=(--set 'dispersion.wind_speeds=[7.8, 11.9, 15.6, 19.6, 23.5, 27.4]')
   VMAX=27.4   # 格子の最大の風速
   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.directions=36 --out runs/grid_$m
   done

期待する結果： 各出力先の ``dispersion.csv`` と ``dispersion_meta.json`` が書かれ、降下モードごとの最大着地距離が表示される。

手順 2　格子を細かくして、最大値が変わらないことを確かめる
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

方位を増やした格子（72、360 など）と比べ、さらに最大となったケースの風速と風向の近くを細かくした格子（風速の刻みを小さくし、方位を 360 まで増やす）でも計算する。
最大値が変わる間は細かくし、最も大きい値を使う。最大は風速の上限で起きるとは限らない。弾道落下では、途中の風速で最も遠くなることがある。
多角形と KML の輪郭は着地点を直線で結んだ近似で、方位が粗いと着地点が輪郭の外に出る（:ref:`sec-wind-grid-result`）。保安域は、輪郭ではなく最大着地距離の円か、細かい格子の外側の包絡で考える。

.. code-block:: bash

   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.directions=360 --out runs/grid360_$m
   done
   worst runs/grid360_panel ballistic   # 例：27.4 116

最も遠いケースの風速と風向は、``worst`` で降下モードごとに読む（``profile`` では平均風速と平均風の風向）。
風速の刻みを細かくするには、最も遠いケースの風速の前後の格子の間を細かくした ``wind_speeds`` で計算する（例は 23.5 から 27.4 m/s を約 0.5 m/s 刻みにしたパネル法の格子）。

.. code-block:: bash

   run dispersion "$C" "${P[@]}" "${N[@]}" --set aero.method=panel --aero-table runs/aero_panel --set dispersion.directions=360 --set 'dispersion.wind_speeds=[23.5, 24, 24.5, 25, 25.5, 26, 26.5, 27, 27.4]' --out runs/grid360fine_panel

風速の刻みを細かくした格子で最大値が増えたときは、手順 3 ではその格子の出力先（例では ``runs/grid360fine_panel``）を ``worst`` に与える。

手順 3　機体のばらつきを、最も遠くなる風で飛ばす
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

降下モードと空力の手法ごとに、手順 2 の格子（正常な開傘の格子）で最も遠くなったケースの風を公称の風にして、``mode = "monte_carlo"`` で計算する。回収系は正常な開傘（``N``）である。
故障の場合の最も遠い風は故障ごとに違うので、手順 5 で別に求める。

* ``profile`` では、そのケースの風速と風向を ``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` に与える。表はそのケースと同じ倍率と角度で拡大・回転され、公称の飛翔は格子のそのケースと同じ風になる。
  ``constant``、``power``、``log`` では、逆に ``wind.speed`` と ``wind.direction_deg`` に与える。
* 風は格子で調べたので、``wind_speed`` と ``wind_direction_deg`` の標準偏差は 0 にし、推力、質量、空力係数、レールの向きなどの機体のばらつきだけを与える。
* 機体のばらつきで最も遠くなる風向がずれることがあるので、上限の風速でほかの風向（たとえば 30° ごと）も計算する。
  風の格子は公称の機体だけを飛ばすので、機体のばらつきと強い風が重なった着地点は、この手順でしか出ない。
* サンプル数は :math:`10^4` 以上にする。最大値はサンプル数とともに大きくなるので、P99 と P99.9 と最大値を併記して報告し、サンプル数を 2 から 4 倍にした計算と比べる（:ref:`tbl-safety-samples`）。

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

期待する結果： 各 ``dispersion_summary.json`` の ``wind_speed``、``wind_direction_deg``、``wind_scale`` が、意図した公称の風になっている。

手順 4　予報の風でモンテカルロ解析を行い、標準偏差の根拠を残す
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

当日の予報の表そのものを公称の風にして、風のばらつき（予報の誤差）も含めて計算する。回収系は正常な開傘（``N``）である。
``profile`` の風速の標準偏差は平均風速 :math:`W` の周りの揺らぎで、表全体を拡大する。
標準偏差は利用者が決める値で、既定値は目安にすぎない。推力は燃焼試験のばらつき、空力係数は手法どうしの差、風は予報の誤差のように、根拠を決めて設定する（:ref:`page-reference-config-dispersion`）。

.. code-block:: bash

   for m in barrowman panel; do
     run dispersion "$C" "${P[@]}" "${N[@]}" \
         --set aero.method=$m --aero-table runs/aero_$m \
         --set dispersion.mode=monte_carlo \
         --set dispersion.monte_carlo.samples=10000 \
         --out runs/mc_forecast_$m
   done

手順 5　回収系の故障を飛ばす
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

弾道落下（パラシュートが開かない場合）は、手順 1 から 4 で常に計算されるので、その最大値を含める。弾道落下は回収系の設定によらないので、故障の計算でも手順 1 から 4 と同じ値になる。
単一のパラシュートの機体では、この手順はこれだけである。
二段開傘の機体では、さらに次の 2 つの故障を、手順 2 と同じ格子と、その格子でパラシュート降下が最も遠くなった風でのモンテカルロ解析（手順 3 と同じばらつき）で計算する。

* **メインが頂点で開く**\ （配列 ``A``）: ``main_altitude`` を頂点より高くする（例えば 100000）。メインがドローグと同時に開き、``recovery.main_altitude = … m is above the apogee`` の警告が出る。
* **メインが開かず、ドローグだけで降りる**: ``N`` を加えない（``main_cd_s`` と ``main_altitude`` を指定しない）。

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

風による流されは、降下時間が長いほど大きい。メインが頂点で開くと降下時間が最も長く、着地点は風下へ最も遠い。
ドローグだけの降下は、ドローグの抗力面積が小さく降下が速ければ、正常な二段開傘より **近く** に落ちる（着地速度は大きい）。ドローグが大きく降下が遅ければ遠くなるので、どちらになるかは降下時間による。思い込みで除かず、計算して大きいほうを取る。
パラシュートが開くときの対気速度と動圧は ``summary.json`` の ``drogue_deploy_speed`` ``drogue_deploy_q`` ``main_deploy_speed`` ``main_deploy_q`` に出る。
50 m/s または 2 kPa を超える開傘には警告が出る（:ref:`IY-O518`）。着地点はパラシュートが壊れないものとして計算しているので、この警告が出る開傘は、パラシュートが壊れて弾道落下に近い降下になりうるものとして扱う。

手順 6　空力の手法の違いを含める
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

空力係数は実測と比較しておらず（:ref:`page-theory-limitations`）、手法の差は最大着地距離に大きく効く。
上のコマンドのように、``aero.method = "barrowman"`` と ``"panel"`` の両方で手順 1 から 5 を計算し、大きいほうの最大値を使う。
公称の飛翔の着地点の差（サンプルの弾道落下で約 69 m）は小さくても、風の格子とモンテカルロの最大値の差は大きくなりうる。
``cn_scale`` の標準偏差は 1 つの手法の中の不確かさで、手法の差はこの手順で含める。

手順 7　終了ステータスと警告を確かめ、半径を決める
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``runs/status.txt`` が空であること（すべてのコマンドの終了ステータスが 0 であること）を確かめる。ここに書かれた計算の結果は保安の判断に使わない（:ref:`sec-exit-status`）。
着地しなかったケース、失敗したケース、静的に不安定なケースや安定を判定できないケース、係数表の迎角の範囲外の引き出しがあると、終了ステータス 3 になり、標準エラー出力に ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行が出る。
失敗や着地しないケースがあるときの最大着地距離は下限である（:ref:`sec-wind-grid-failed`）。
警告は ``-q`` でも表示され、``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` に残る。開傘の速さ、静安定余裕の不足、``power`` と ``log`` の風の上空への外挿、表の拡大率、横向きに落ちた弾道落下（:ref:`sec-dispersion-broadside`）も見る。
特に ``… reach above the top of the wind profile (… m), up to … m: the wind above is held at the top value`` の警告が出たら、表の最上端より上の風は表していないので、最も高い頂点より上まで続く表で計算し直す（:ref:`sec-wind-profile`、:ref:`IY-O512`）。

各計算の最大着地距離は、風の格子では ``dispersion.csv``、モンテカルロ解析では ``dispersion_summary.json`` の ``max_landing_distance`` にある。
次のスクリプトは、計算ごとに最大着地距離（モンテカルロ解析では P99 と P99.9 も）と、横向きに落ちた弾道落下の数 ``broadside_runs``、警告 ``warnings``\ （風の格子では ``dispersion_meta.json`` から）をまとめて出す。

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

半径は、手順 1 から 6 のすべての計算の、弾道落下とパラシュート降下の最大着地距離の最大に、余裕を足した値とする。

.. math::

   R = \max\bigl(\text{手順 1〜6 のすべての計算の最大着地距離}\bigr) + \text{余裕}

余裕には、風は定常で突風や乱流を含まないこと、地形を考えず着地を射点標高の面で判定すること、空力係数が未検証であることを見込む（:ref:`page-theory-limitations`）。余裕の大きさは利用者が決める。

確認
------

* ``runs/status.txt`` が空である。
* すべての ``dispersion_summary.json`` で ``complete`` が ``true`` で、``unstable`` と ``stability_unknown_runs`` が 0 である。
* 風の格子の最大値が、方位と風速の刻みを細かくしても変わらない（手順 2）。
* モンテカルロ解析のサンプル数が :math:`10^4` 以上で、P99、P99.9、最大値を併記している。サンプル数を 2 から 4 倍にしても P99 と P99.9 が変わらない。
  最大値は収まらないので、サンプル数とともに記録する。最大値を決めている飛翔（``max_landing_distance_sample`` の行）のばらつきを ``dispersion_mc.csv`` で見て、その組合せが起こりうるものかを判断する（:ref:`page-theory-dispersion`）。
* 手順 3 と 5 は、上限の風、機体のばらつきの裾、回収系の故障という、それぞれ起こりにくい条件を重ねる。同時に起こる確率は小さく、得られる半径は意図して保守的なものである。
  どの条件までを重ねるかは、打上げを行う団体が決める。
* ``sim``、風の格子、モンテカルロ解析で ``[sim]`` を含む設定（``[dispersion]``、``[output]``、``[resources]`` を除く）が同じである。違うと、KML で飛翔経路・風の格子・モンテカルロの層が 1 つにそろわない。手順の各計算は ``--out`` で出力先を分けるので、KML も計算ごとに別になる。

計算例
~~~~~~

サンプル（射角 85°、射方位 270°）について、手順 1 から 4 を計算した最大着地距離を :numref:`tbl-safety-example` に示す。
再現できるように、高度別の表ではなくサンプル設定の ``power`` の風（風速は地上 2 m、上限 7 m/s）で、``P=(--set sim.integrator=rk45)``、``N=()``、``G=(--set 'dispersion.wind_speeds=[1,2,3,4,5,6,7]')``、``VMAX=7`` として計算した。
手順 3 の風は ``--set wind.speed=7 --set wind.direction_deg=270`` のように ``[wind]`` の ``speed`` と ``direction_deg`` で与える。
モンテカルロ解析は :math:`10^4` サンプル、``seed = 1`` である。

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
   * - 手順 2：細分した格子（3〜5 m/s を 0.25 m/s 刻み、5〜7 m/s を 0.5 m/s 刻み、360 方位）、部品積み上げ法
     - 1549.6（7 m/s、270°）
     - 14315.6（7 m/s、90°）
   * - 手順 2：同、パネル法
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

見る点は次のとおりである。

* 格子の最大値は、方位数や風速の刻みを細かくしても変わらない。
* 手順 3 の最大（パラシュート降下の 28927.8 m）は、格子の最大（14315.6 m）の約 2 倍である。この着地点は、公称の機体を飛ばす格子にも、予報の風の周りのモンテカルロ解析にも現れず、手順 3 でしか出ない。
* 空力の手法による最大値の差は、弾道落下で、格子が 4 %、手順 3 が 1.5 % である。
* 二段開傘の機体では、メインが頂点で開く故障が半径を決めることがある。降下時間が正常な開傘の約 3 倍になるからである。ドローグだけの降下は、正常な開傘より近くに落ちることがある。

半径は :math:`\max(3993.0,\ 28927.8)=28927.8` m に余裕を足した値になる。余裕を 10 % とみなせば約 31821 m である（10 % は計算例であり、推奨値ではない）。
手順 3 を省くと最大は 19116.1 m で、手順 3 を含めた値の約 2/3 にとどまる。
サンプル数と最大値の関係は :numref:`tbl-safety-samples`、高度別の表や二段開傘の計算の読み方は :ref:`page-theory-dispersion` にある。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 42 58

   * - 症状
     - 対処
   * - ``runs/status.txt`` に行がある
     - その計算の結果は保安の判断に使わない。行の終了ステータスと、その計算の標準エラー出力の ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行を読み、直して再実行する（:ref:`page-howto-warnings-errors`）
   * - ``INCOMPLETE RESULTS: [IY-O028]``、``[IY-O029]``
     - 着地しないか失敗したケースがある。最大着地距離は下限である。``sim.max_time`` を延ばすか、失敗の理由を直す（:ref:`sec-wind-grid-failed`、:ref:`sec-mc-failure`）
   * - ``RESULTS NOT USABLE: [IY-O025]``
     - 静的に不安定なケースがある。静安定余裕を増やすか、重心や空力係数の標準偏差を見直す（:ref:`sec-dispersion-unstable`）
   * - ``RESULTS NOT USABLE: [IY-O022]``、``[IY-O023]``、``[IY-O024]``
     - 飛翔が係数表の範囲を外れた。表の範囲を広げて作り直す（:ref:`IY-O024`）
   * - ``warning[IY-O536]``
     - 横向きに落ちた弾道落下がある。距離は除かず、空力の手法を替えた計算（手順 6）と比べて大きいほうを使う（:ref:`sec-dispersion-broadside`）
   * - ``warning[IY-O512]``
     - 頂点が風の表の最上端より上である。最も高い頂点より上まで続く表で計算し直す（:ref:`IY-O512`）
   * - ``warning[IY-C502]: wind.speed is ignored …``
     - ``profile`` では ``speed`` と ``direction_deg`` は使われない。結果に影響しない。消してよい（:ref:`IY-C502`）
   * - 計算に時間がかかりすぎる
     - ``sim.integrator`` を ``rk45`` にし、サンプル数を段階的に増やす。計算時間の目安は :ref:`page-vv-performance` にある

関連
------

* 落下分散の実行と結果の読み方：:ref:`page-howto-dispersion-run`
* この手順の根拠（サンプル数と最大値、最悪の重ね合わせ、ばらつきの与え方）：:ref:`page-theory-dispersion`
* 設定キー：:ref:`page-reference-config-dispersion`
* 条件を変えて繰り返す（``--set`` ``--out`` ``--aero-table``）：:ref:`page-howto-sweeps`
* 風の設定：:ref:`page-howto-environment`
* 仮定と適用範囲：:ref:`page-theory-limitations`
