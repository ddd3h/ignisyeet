コマンドと出力（IY-O）
==========================

コマンドの引数、出力ディレクトリ、飛行・分散計算の警告と結果の使用可否が出力するエラーとワーニングの一覧である。エラーは ``error[IY-Onnn]: …``、ワーニングは ``warning[IY-Onnn]: …`` の形で表示する（クワイエットモードではワーニングも同じ形で標準エラーへ出す）。終了コード 3 の結果は ``ignisyeet: RESULTS NOT USABLE: [IY-Onnn] …`` または ``ignisyeet: INCOMPLETE RESULTS: [IY-Onnn] …`` の形で表示する。ワーニングの文字列は ``[IY-Onnn]`` を先頭に付けたまま ``summary.json`` などの ``warnings`` 配列にも書く。山括弧 ``<…>`` は実行時の値に置き換わる箇所である。

.. _IY-O001:

IY-O001
-------

重大度: エラー

メッセージ::

    error[IY-O001]: --descent parachute needs recovery.enabled = true

意味
    ``--descent parachute`` を指定したが回収装置が無効である。

主な原因
    ``recovery.enabled = false`` のまま ``sim --descent parachute`` を実行した。

対処
    ``[recovery]`` を有効にするか、``--descent ballistic`` を使う。

.. _IY-O002:

IY-O002
-------

重大度: エラー

メッセージ::

    error[IY-O002]: --aero-table cannot be used with aero.method = "table": the table is aero.table

意味
    ``--aero-table`` を ``aero.method = "table"`` と同時に使った。

主な原因
    外部表（``aero.table``）を使う設定で別の表を指定した。

対処
    ``--aero-table`` を外す。

.. _IY-O003:

IY-O003
-------

重大度: エラー

メッセージ::

    error[IY-O003]: --aero-table and --force cannot be used together

意味
    ``--aero-table`` と ``--force`` を同時に使った。

主な原因
    再利用する表を作り直すよう求めた。

対処
    どちらかを外す。

.. _IY-O004:

IY-O004
-------

重大度: エラー

メッセージ::

    error[IY-O004]: --aero-table: <value> was built for another rocket or other aero settings (cache key <value> vs <hash> of this configuration); 

意味
    ``--aero-table`` の表が、この設定の機体と空力設定で作られたものではない。

主な原因
    形状や空力の設定を変えた後に古い表を指定した。

対処
    ``ignisyeet aero`` でこの設定の表を作るか、``--aero-table`` を外す。

.. _IY-O005:

IY-O005
-------

重大度: エラー

メッセージ::

    error[IY-O005]: body length <length> m is outside the plausible <value>-<value> m range: check the STL units and rocket.stl_scale (now <value>; 0.001 converts millimetres to metres)

意味
    機体の全長が機体として妥当な範囲（0.1 から 20 m）にない。

主な原因
    ``rocket.stl_scale`` の誤り（mm の STL に 1 を指定した等）。

対処
    ``rocket.stl_scale`` を確認する（mm から m は 0.001）。

関連
    :doc:`../geometry`

.. _IY-O006:

IY-O006
-------

重大度: エラー

メッセージ::

    error[IY-O006]: rocket.cg_dry = <cg> m is outside the body: it is measured aft of the nose tip and the body is <length> m long (check rocket.stl_scale and the unit of cg_dry)

意味
    ``rocket.cg_dry`` が機体の外にある。

主な原因
    ``rocket.cg_dry`` の単位や基準点の誤り（基準は機首先端、後ろ向きに正）。

対処
    ``rocket.cg_dry`` と ``rocket.stl_scale`` を確認する。

.. _IY-O007:

IY-O007
-------

重大度: エラー

メッセージ::

    error[IY-O007]: the motor (<motor_length> m) is longer than the body (<length> m): check motor.eng (the motor length in its header, in millimetres) and rocket.stl_scale

意味
    モーターが機体より長い。

主な原因
    ``.eng`` の長さの単位（mm）の誤り、``rocket.stl_scale`` の誤り。

対処
    ``motor.eng`` の長さと ``rocket.stl_scale`` を確認する。

.. _IY-O008:

IY-O008
-------

重大度: エラー

メッセージ::

    error[IY-O008]: <value> puts the motor outside the body: the motor is <motor_length> m long and the body <length> m, so motor.aft_x (the motor's aft end, measured aft of the nose tip) must be between <motor_length> and <length> m

意味
    モーターが機体の外に出る位置にある。

主な原因
    ``motor.aft_x`` の誤り。

対処
    ``motor.aft_x`` をモーター長以上、機体長以下にする。

.. _IY-O009:

IY-O009
-------

重大度: エラー

メッセージ::

    error[IY-O009]: CFD tools are missing

意味
    CFD ツールが見つからない（``cfd-check``）。

主な原因
    SU2、MPI、gmsh のいずれかが入っていない。

対処
    ``install.sh --cfd`` でインストールする。直前に表示した ``IY-F`` のメッセージも参照する。

関連
    :doc:`../cfd`

.. _IY-O011:

IY-O011
-------

重大度: エラー

メッセージ::

    error[IY-O011]: all <errored> wind-grid cases failed with an error (first: <value>); the results are in <value>

意味
    風速グリッドの全ケースがエラーで失敗した。

主な原因
    設定や空力表の不備で全ケースの積分が失敗する。

対処
    メッセージ内の最初のエラーを確認する。結果の CSV にケースごとのエラーが残る。

関連
    :ref:`sec-wind-grid-failed`

.. _IY-O012:

IY-O012
-------

重大度: エラー

メッセージ::

    error[IY-O012]: aero.table is required when aero.method = "table"

意味
    ``aero.method = "table"`` なのに ``aero.table`` がない。

主な原因
    表のパスを書き忘れた。

対処
    ``aero.table`` に外部表のパスを書く。

.. _IY-O013:

IY-O013
-------

重大度: エラー

メッセージ::

    error[IY-O013]: cannot read motor file <value>

意味
    モーターファイル（``motor.eng``）を読めない。

主な原因
    パスの誤り、読み取り権限がない。

対処
    ``motor.eng`` のパスと権限を確認する。

.. _IY-O014:

IY-O014
-------

重大度: エラー

メッセージ::

    error[IY-O014]: cannot read the CAD file <value>

意味
    CAD ファイルを読めない（空力表のキャッシュキーの計算）。

主な原因
    ``aero.cfd`` で指定した CAD ファイルがない。

対処
    パスと権限を確認する。

.. _IY-O016:

IY-O016
-------

重大度: エラー

メッセージ::

    error[IY-O016]: cannot copy <value> to <value>

意味
    空力表と付随ファイルを出力ディレクトリへ複写できない。

主な原因
    出力先に書けない、容量が足りない。

対処
    出力先の権限と空き容量を確認する。

.. _IY-O020:

IY-O020
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O020] the rocket is statically UNSTABLE (min static margin <m> cal<value>) (exit status 3)

意味
    機体が静的に不安定で、結果を使えない（終了コード 3）。

主な原因
    圧力中心が重心より前にある。

対処
    重心を前へ寄せるか、フィンを大きくする。

関連
    :doc:`../safety`

.. _IY-O021:

IY-O021
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O021] static stability could not be evaluated (<why>) (exit status 3)

意味
    静的安定性を評価できず、結果を使えない（終了コード 3）。

主な原因
    空力表の横力係数が 0 など、静的余裕を計算できない。

対処
    括弧内の理由に従って空力表を確認する。

関連
    :doc:`../safety`

.. _IY-O022:

IY-O022
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O022] <value> of <value> lookups beyond the aero table's Mach range (max Mach <value> > mach_max <mach_max>): coefficients are held at the edge; build the table up to at least Mach <need> (exit status 3)

意味
    空力表のマッハ数の上限を超える参照があり、結果を使えない（終了コード 3）。

主な原因
    表の ``aero.mach_max`` が飛行の最大マッハ数より小さい。

対処
    メッセージが示すマッハ数まで表を作る。

関連
    :doc:`../aerodynamics`

.. _IY-O023:

IY-O023
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O023] <value> of <value> lookups below the aero table's Mach range (first Mach <mach_min>): coefficients are held at the edge; build the table down to Mach 0 (exit status 3)

意味
    空力表のマッハ数の下限を下回る参照があり、結果を使えない（終了コード 3）。

主な原因
    表が 0 から始まっていない。

対処
    表をマッハ 0 から作る。

.. _IY-O024:

IY-O024
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O024] <value> of <value> lookups beyond the aero table's angle range (max |alpha| <value> deg > alpha_max <alpha_max> deg): the descent uses extrapolated coefficients; <fix> (exit status 3)

意味
    空力表の迎角の範囲を超える参照があり、結果を使えない（終了コード 3）。

主な原因
    降下中の迎角が表の最大迎角より大きい。

対処
    表を迎角 90 度（一般表は 180 度）まで作る。

関連
    :doc:`../aerodynamics`

.. _IY-O025:

IY-O025
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O025] <value>: <hint><more> (exit status 3)

意味
    分散計算に静的に不安定なサンプルがあり、結果を使えない（終了コード 3）。

主な原因
    摂動で静的余裕が負になる。

対処
    静的余裕を増やすか、重心と空力のシグマを小さくする。

関連
    :ref:`sec-dispersion-unstable`

.. _IY-O026:

IY-O026
-------

重大度: エラー

メッセージ::

    ignisyeet: RESULTS NOT USABLE: [IY-O026] <value> (exit status 3)

意味
    分散計算に静的安定性を評価できないサンプルがあり、結果を使えない（終了コード 3）。

主な原因
    空力表から静的余裕を計算できない。

対処
    括弧内の理由に従って空力表を確認する。

関連
    :ref:`sec-dispersion-unstable`

.. _IY-O027:

IY-O027
-------

重大度: エラー

メッセージ::

    ignisyeet: INCOMPLETE RESULTS: [IY-O027] the flight did not land within sim.max_time = <value> s (exit status 3)

意味
    飛行が ``sim.max_time`` までに着地しなかった（終了コード 3）。

主な原因
    降下が遅い、開傘しなかった。

対処
    ``sim.max_time`` を延ばすか降下条件を確認する。

.. _IY-O028:

IY-O028
-------

重大度: エラー

メッセージ::

    ignisyeet: INCOMPLETE RESULTS: [IY-O028] <failed> of <value> wind-grid cases did not land (exit status 3)

意味
    風速グリッドに着地しなかったケースがある（終了コード 3）。

主な原因
    降下が遅い、``sim.max_time`` が短い。

対処
    ``sim.max_time`` と降下条件を確認する。

関連
    :ref:`sec-wind-grid-failed`

.. _IY-O029:

IY-O029
-------

重大度: エラー

メッセージ::

    ignisyeet: INCOMPLETE RESULTS: [IY-O029] <failed> Monte Carlo runs did not land or failed; the statistics are incomplete (exit status 3)

意味
    モンテカルロに着地しなかった、または失敗したサンプルがある（終了コード 3）。

主な原因
    降下が遅い、摂動で失敗した。

対処
    ``dispersion_mc.csv`` の status を確認する。

関連
    :ref:`sec-mc-failure`

.. _IY-O030:

IY-O030
-------

重大度: エラー

メッセージ::

    error[IY-O030]: cannot create <value>

意味
    出力ディレクトリを作れない。

主な原因
    親ディレクトリがない、権限がない、同名のファイルがある。

対処
    ``output.dir`` または ``--out`` を確認する。

.. _IY-O031:

IY-O031
-------

重大度: エラー

メッセージ::

    error[IY-O031]: another ignisyeet run is using <0> (lock file <0>/.ignisyeet.lock, pid 1); wait for it or use a different --out (if no other run is active, delete <0>/.ignisyeet.lock)

意味
    別の ignisyeet がこの出力ディレクトリを使っている。

主な原因
    同じ出力先で複数のコマンドを同時に実行した。ロックファイルが残っている。

対処
    実行中のコマンドが終わるまで待つか、``--out`` で別のディレクトリを使う。他に実行中のものがなければロックファイルを削除する。

関連
    :ref:`sec-output-lock`

.. _IY-O032:

IY-O032
-------

重大度: エラー

メッセージ::

    error[IY-O032]: cannot create the lock file <value>

意味
    ロックファイルを作れない。

主な原因
    出力先に書き込めない。

対処
    出力先の権限を確認する。

.. _IY-O033:

IY-O033
-------

重大度: エラー

メッセージ::

    error[IY-O033]: cannot write <value>

意味
    ファイルを書き込めない。

主な原因
    出力先に書けない、容量が足りない。

対処
    出力先の権限と空き容量を確認する。

.. _IY-O040:

IY-O040
-------

重大度: エラー

メッセージ::

    error[IY-O040]: expected 'ballistic' or 'parachute'

意味
    ``--descent`` の値が ``ballistic`` でも ``parachute`` でもない。

主な原因
    綴りの誤り。

対処
    ``ballistic`` か ``parachute`` を指定する。

.. _IY-O500:

IY-O500
-------

重大度: ワーニング

メッセージ::

    warning[IY-O500]: <key> = <step> is coarse (> <COARSE_MACH_STEP>): the coefficients are interpolated linearly between the Mach rows, which smooths over the transonic drag rise and shifts the apogee (0.02 is typical)

意味
    空力表のマッハ刻みが粗い（0.1 超）。

主な原因
    ``aero.mach_step`` または ``aero.general_mach_step`` が大きい。

対処
    0.02 程度にする。遷音速の抗力上昇がなまり、頂点高度がずれる。

関連
    :doc:`../aerodynamics`

.. _IY-O501:

IY-O501
-------

重大度: ワーニング

メッセージ::

    warning[IY-O501]: aero.alpha_step_deg = <value> is coarse (> <COARSE_ALPHA_STEP_DEG> deg): the normal force and centre of pressure are interpolated linearly between the rows (1 deg is typical)

意味
    空力表の迎角刻みが粗い（5 度超）。

主な原因
    ``aero.alpha_step_deg`` が大きい。

対処
    1 度程度にする。

.. _IY-O502:

IY-O502
-------

重大度: ワーニング

メッセージ::

    warning[IY-O502]: body length <length> m is outside the plausible <value>-<value> m range: check rocket.stl_scale (now <value>; 0.001 converts millimetres to metres)

意味
    機体の全長が妥当な範囲（0.1 から 20 m）にない。

主な原因
    ``rocket.stl_scale`` の誤り。

対処
    ``rocket.stl_scale`` を確認する。

.. _IY-O503:

IY-O503
-------

重大度: ワーニング

メッセージ::

    warning[IY-O503]: body diameter <diameter> m is outside the plausible <value>-<value> m range: check rocket.stl_scale (now <value>)

意味
    機体の直径が妥当な範囲（0.005 から 3 m）にない。

主な原因
    ``rocket.stl_scale`` の誤り。

対処
    ``rocket.stl_scale`` を確認する。

.. _IY-O504:

IY-O504
-------

重大度: ワーニング

メッセージ::

    warning[IY-O504]: THE FLIGHT DID NOT LAND within sim.max_time = <value> s: there is no landing point; raise sim.max_time or check the descent (parachute, drag area, launch angle)

意味
    飛行が ``sim.max_time`` までに着地せず、着地点がない。

主な原因
    降下が遅い、開傘しなかった。

対処
    ``sim.max_time`` を延ばすか降下条件を確認する。

.. _IY-O505:

IY-O505
-------

重大度: ワーニング

メッセージ::

    warning[IY-O505]: static stability could not be evaluated (<why>): the results are not usable until it is

意味
    静的安定性を評価できていない。

主な原因
    空力表から静的余裕を計算できない。

対処
    括弧内の理由に従う。評価できるまで結果は使えない。

.. _IY-O506:

IY-O506
-------

重大度: ワーニング

メッセージ::

    warning[IY-O506]: static margin <value> cal<at> is negative: the centre of pressure is forward of the CG, the rocket is UNSTABLE

意味
    静的余裕が負で、機体が不安定である。

主な原因
    圧力中心が重心より前にある。

対処
    重心を前へ寄せるか、フィンを大きくする。

関連
    :doc:`../safety`

.. _IY-O507:

IY-O507
-------

重大度: ワーニング

メッセージ::

    warning[IY-O507]: minimum static margin <value> cal<at> is below 1.0 cal: low static margin

意味
    最小の静的余裕が 1.0 cal 未満である。

主な原因
    重心が後ろ、フィンが小さい。

対処
    静的余裕を 1.5 cal 以上にする。

関連
    :doc:`../safety`

.. _IY-O508:

IY-O508
-------

重大度: ワーニング

メッセージ::

    warning[IY-O508]: minimum static margin <value> cal<at> is below the recommended 1.5 cal (marginal)

意味
    最小の静的余裕が推奨の 1.5 cal 未満である。

主な原因
    同上。

対処
    静的余裕を増やす。

.. _IY-O509:

IY-O509
-------

重大度: ワーニング

メッセージ::

    warning[IY-O509]: the ballistic descent fell broadside (angle of attack 60 to 120 deg for <value> s): the landing depends on the large-angle aerodynamics, which are not validated

意味
    弾道降下が横向き（迎角 60 から 120 度）で落ちた。

主な原因
    降下中の姿勢が安定しない。

対処
    着地点は大迎角の空力に依存し、その空力は検証していない。降下方式を見直す。

関連
    :ref:`sec-dispersion-broadside`

.. _IY-O510:

IY-O510
-------

重大度: ワーニング

メッセージ::

    warning[IY-O510]: roll resonance: the roll rate crosses the resonance rate of the pitch oscillation (<value> rad/s) at t = <value> s, Mach <value><value>; the lateral CG offset, thrust misalignment or asymmetric aero moment can amplify the angle of attack there

意味
    ロール角速度がピッチ振動の共振角速度と交差する。

主な原因
    ロール率の上昇がピッチ固有振動数を通過する。

対処
    重心の横ずれ、推力の傾き、非対称なモーメントが迎角を増幅する可能性がある。

.. _IY-O511:

IY-O511
-------

重大度: ワーニング

メッセージ::

    warning[IY-O511]: recovery.main_altitude = <m> m is above the apogee (<value> m): the main opens together with the drogue

意味
    ``recovery.main_altitude`` が頂点高度以上で、主傘がドローグと同時に開く。

主な原因
    主傘の開傘高度の設定が高すぎる。

対処
    頂点より低い高度にする。

.. _IY-O512:

IY-O512
-------

重大度: ワーニング

メッセージ::

    warning[IY-O512]: apogee <value> m is above the top of the wind profile (<top> m): the wind above <top> m is held at the top value

意味
    頂点高度が風プロファイルの最上段より高い。

主な原因
    プロファイルが低い高度までしかない。

対処
    最上段の風が上空でも一定とみなされる。上空までの風を用意する。

関連
    :ref:`sec-wind-profile`

.. _IY-O513:

IY-O513
-------

重大度: ワーニング

メッセージ::

    warning[IY-O513]: the wind profile starts at <bottom> m: the wind below is held at the lowest row

意味
    風プロファイルの最下段が地面より高い。

主な原因
    プロファイルが 0 m の行から始まらない。

対処
    最下段より下の風は最下段の値で一定とみなされる。0 m の行を加える。

.. _IY-O514:

IY-O514
-------

重大度: ワーニング

メッセージ::

    warning[IY-O514]: apogee <value> m is above <WIND_VALID_HEIGHT> m, the validity height of the <value> wind profile: the profile is extrapolated there, and wind.speed = <value> m/s is the speed at wind.ref_height = <value> m, so the winds aloft and the landing point are model-dependent

意味
    頂点高度が風モデルの有効高さ（1000 m）より高い。

主な原因
    べき乗則や対数則を高高度まで外挿している。

対処
    上空の風と着地点はモデル依存になる。風プロファイルを使う。

関連
    :doc:`../environment`

.. _IY-O515:

IY-O515
-------

重大度: ワーニング

メッセージ::

    warning[IY-O515]: rail exit speed <value> m/s is below <MIN_RAIL_EXIT_SPEED> m/s: the fins have little authority at rail exit and the rocket is easily blown off the rail line; use a longer rail or a higher thrust-to-weight ratio

意味
    レール離脱速度が 15 m/s 未満である。

主な原因
    レールが短い、推力重量比が低い。

対処
    レールを長くするか、推力重量比を上げる。

.. _IY-O516:

IY-O516
-------

重大度: ワーニング

メッセージ::

    warning[IY-O516]: the wind angle of attack at rail exit is <value> deg (wind <wind_speed> m/s at <h> m, rail exit speed <v> m/s), above <MAX_RAIL_EXIT_WIND_ALPHA_DEG> deg: the rocket will weathercock strongly off the rail

意味
    レール離脱時の風の迎角が 15 度を超える。

主な原因
    横風に対してレール離脱速度が低い。

対処
    レール離脱速度を上げる。

.. _IY-O517:

IY-O517
-------

重大度: ワーニング

メッセージ::

    warning[IY-O517]: the motor still burns at apogee + recovery.delay (t = <value> s, thrust <value> N): the <first> deployment is delayed to burnout at <t> s

意味
    開傘時刻にモーターがまだ燃焼しており、開傘を燃え尽きまで遅らせた。

主な原因
    ``recovery.delay`` が短い。

対処
    ``recovery.delay`` を延ばす。

.. _IY-O518:

IY-O518
-------

重大度: ワーニング

メッセージ::

    warning[IY-O518]: the <what> opens at <value> m/s (q = <value> kPa): a canopy is unlikely to survive; the landing point assumes it does - check <key>

意味
    開傘時の速度または動圧が大きく、パラシュートが耐えにくい。

主な原因
    ``recovery.delay`` や ``recovery.main_altitude`` の設定が不適切である。

対処
    メッセージが示す設定を見直す。

.. _IY-O519:

IY-O519
-------

重大度: ワーニング

メッセージ::

    warning[IY-O519]: parachute descent landed at <value> m/s: the parachute did not deploy before landing (recovery.delay = <value> s)

意味
    パラシュートが着地までに開かなかった。

主な原因
    ``recovery.delay`` が長すぎる。

対処
    ``recovery.delay`` を短くする。

.. _IY-O520:

IY-O520
-------

重大度: ワーニング

メッセージ::

    warning[IY-O520]: parachute descent landed at <value> m/s, which is above <MAX_PARACHUTE_LANDING_SPEED> m/s: check recovery.cd_s

意味
    パラシュート降下の着地速度が 15 m/s を超える。

主な原因
    ``recovery.cd_s`` が小さい。

対処
    ``recovery.cd_s`` を確認する。

.. _IY-O521:

IY-O521
-------

重大度: ワーニング

メッセージ::

    warning[IY-O521]: wind profile scaled by a factor of <factor> (<what> <v> m/s vs profile mean <value> m/s)<aloft>

意味
    風プロファイルを大きく拡大または縮小して使った。

主な原因
    風速グリッドやモンテカルロの風速がプロファイルの平均風速から大きく離れている。

対処
    上空の風が非現実的な値になっていないか確認する。

関連
    :ref:`sec-wind-scaling`

.. _IY-O522:

IY-O522
-------

重大度: ワーニング

メッセージ::

    warning[IY-O522]: <value> of <value> <what> reach above the top of the wind profile (<top> m), up to <highest> m: the wind above is held at the top value

意味
    飛行の一部が風プロファイルの最上段より上に達した。

主な原因
    プロファイルが低い高度までしかない。

対処
    最上段より上の風は一定とみなされる。

.. _IY-O523:

IY-O523
-------

重大度: ワーニング

メッセージ::

    warning[IY-O523]: dispersion.monte_carlo.magnus_scale = <value> has no effect: the aero table has no Magnus slopes (cnpa = cmpa = 0; no built-in method estimates them, supply them in an external table)

意味
    ``dispersion.monte_carlo.magnus_scale`` が効かない。

主な原因
    空力表にマグヌスの傾きがない。

対処
    外部表にマグヌス項を入れるか、``magnus_scale`` を 0 にする。

.. _IY-O524:

IY-O524
-------

重大度: ワーニング

メッセージ::

    warning[IY-O524]: dispersion.monte_carlo.lag_scale = <value> has no effect: the aero table has no lag sums (lag_l0 = lag_l1 = 0)

意味
    ``dispersion.monte_carlo.lag_scale`` が効かない。

主な原因
    空力表にラグの和がない。

対処
    表にラグ列を入れるか、``lag_scale`` を 0 にする。

.. _IY-O525:

IY-O525
-------

重大度: ワーニング

メッセージ::

    warning[IY-O525]: the external aero table has no lag/Magnus columns: the apparent-mass (lag) sums were filled from the geometry (component build-up); Magnus force and induced roll are 0. If its damping sums hold a measured or CFD Cmq + Cmalphadot (which already contains these terms), write lag_l0 and lag_l1 columns of 0 so they are not counted twice

意味
    外部表にラグ（見かけ質量）とマグヌスの列がなく、形状から補った。

主な原因
    古い列構成の外部表。

対処
    減衰の和に実測や CFD の値が入っているなら、``lag_l0`` と ``lag_l1`` を 0 で書いて二重計上を避ける。

.. _IY-O526:

IY-O526
-------

重大度: ワーニング

メッセージ::

    warning[IY-O526]: the aero table has an induced roll cl4 but no fin_count in its sidecar: the sin(4 phi) form of four fins is assumed

意味
    外部表に誘導ロール ``cl4`` があるが、サイドカーに ``fin_count`` がない。

主な原因
    表のサイドカーに枚数が書かれていない。

対処
    4 枚の sin(4φ) 形を仮定する。サイドカーに ``fin_count`` を書く。

.. _IY-O527:

IY-O527
-------

重大度: ワーニング

メッセージ::

    warning[IY-O527]: the external aero table has no lag/Magnus columns and the geometry is unavailable: the flight has no apparent-mass (lag) terms, Magnus force or induced roll

意味
    外部表にラグ・マグヌスの列がなく、形状も使えないため、これらの項を 0 にした。

主な原因
    形状が読めない外部表。

対処
    形状を用意するか、列を持つ表を使う。

.. _IY-O528:

IY-O528
-------

重大度: ワーニング

メッセージ::

    warning[IY-O528]: the external aero table has no roll terms and the geometry is unavailable (<e>); the rocket will not roll

意味
    外部表にロール項がなく、形状も使えないため、機体はロールしない。

主な原因
    形状が読めない外部表。

対処
    形状を用意するか、ロール項を持つ表を使う。

.. _IY-O529:

IY-O529
-------

重大度: ワーニング

メッセージ::

    warning[IY-O529]: the nominal flight could not be simulated (<e>): stability and wind-model checks were skipped

意味
    公称飛行を計算できず、安定性と風モデルの確認を省いた。

主な原因
    公称の設定で飛行が失敗した。

対処
    括弧内のエラーを直す。

.. _IY-O530:

IY-O530
-------

重大度: ワーニング

メッセージ::

    warning[IY-O530]: <not_landed> of <value> wind-grid cases did not land within sim.max_time = <value> s: they are excluded from the landing distances, so the maximum landing distance is only a lower bound

意味
    風速グリッドに着地しなかったケースがある。

主な原因
    降下が遅い、``sim.max_time`` が短い。

対処
    最大着地距離は下限値になる。``sim.max_time`` と降下条件を確認する。

関連
    :ref:`sec-wind-grid-failed`

.. _IY-O531:

IY-O531
-------

重大度: ワーニング

メッセージ::

    warning[IY-O531]: <errored> of <value> wind-grid cases failed with an error (e.g. <first>): they are excluded from the landing distances, so the maximum landing distance is only a lower bound

意味
    風速グリッドにエラーで失敗したケースがある。

主な原因
    設定や空力表の不備による積分の失敗。

対処
    最大着地距離は下限値になる。メッセージ内のエラーを確認する。

関連
    :ref:`sec-wind-grid-failed`

.. _IY-O532:

IY-O532
-------

重大度: ワーニング

メッセージ::

    warning[IY-O532]: wind-grid case <value> m/s from <value>° (<value>): <e>

意味
    風速グリッドの 1 ケースがエラーで失敗した（最大 3 件を表示）。

主な原因
    ``IY-S`` のエラーを参照する。

対処
    括弧内のエラーに従う。

.. _IY-O533:

IY-O533
-------

重大度: ワーニング

メッセージ::

    warning[IY-O533]: <value> more wind-grid cases failed with an error (see the error column of dispersion.csv)

意味
    表示した以外にもエラーで失敗した風速グリッドのケースがある。

主な原因
    同上。

対処
    ``dispersion.csv`` の ``error`` 列を見る。

.. _IY-O534:

IY-O534
-------

重大度: ワーニング

メッセージ::

    warning[IY-O534]: <c>: they are kept in the statistics (conservative), so the landing distances include tumbling flights

意味
    分散計算に静的に不安定なサンプルがある。

主な原因
    摂動で静的余裕が負になる。

対処
    不安定な飛行も統計に含めるため、着地距離は保守的になる。静的余裕を増やす。

関連
    :ref:`sec-dispersion-unstable`

.. _IY-O535:

IY-O535
-------

重大度: ワーニング

メッセージ::

    warning[IY-O535]: <c>

意味
    分散計算に静的安定性を評価できないサンプルがある。

主な原因
    空力表から静的余裕を計算できない。

対処
    括弧内の理由に従う。

.. _IY-O536:

IY-O536
-------

重大度: ワーニング

メッセージ::

    warning[IY-O536]: <value> of <value> ballistic <what> fell broadside (angle of attack 60 to 120 deg for more than <value> s): the landing depends on the large-angle aerodynamics, which are not validated

意味
    分散計算で弾道降下が横向きに落ちたサンプルがある。

主な原因
    降下中の姿勢が安定しない。

対処
    着地は大迎角の空力に依存し、その空力は検証していない。

関連
    :ref:`sec-dispersion-broadside`

.. _IY-O537:

IY-O537
-------

重大度: ワーニング

メッセージ::

    warning[IY-O537]: <value>: <value> of <value> Monte Carlo samples did not land or failed (<value> did not land within sim.max_time = <value> s): the statistics are INCOMPLETE and the maximum landing distance is only a lower bound

意味
    モンテカルロに着地しなかった、または失敗したサンプルがある。

主な原因
    降下が遅い、摂動で失敗した。

対処
    統計は不完全で、最大着地距離は下限値になる。

関連
    :ref:`sec-mc-failure`

.. _IY-O538:

IY-O538
-------

重大度: ワーニング

メッセージ::

    warning[IY-O538]: <value>: only <value> landed samples: the statistics are rough

意味
    着地したサンプルが 30 未満で、統計が粗い。

主な原因
    ``dispersion.monte_carlo.samples`` が少ない。

対処
    サンプル数を増やす。

関連
    :ref:`sec-mc-percentile`

.. _IY-O539:

IY-O539
-------

重大度: ワーニング

メッセージ::

    warning[IY-O539]: KML: <what> left out (stale: <path> was written with a different configuration; rerun the command to include it)

意味
    KML に含める結果が古い設定で書かれたもので、KML から外した。

主な原因
    設定を変えた後に別のコマンドの結果が残っている。

対処
    コマンドを実行し直して結果を更新する。

.. _IY-O540:

IY-O540
-------

重大度: ワーニング

メッセージ::

    warning[IY-O540]: KML: <what> left out (<reason>)

意味
    KML に含める結果を読めず、KML から外した。

主な原因
    結果ファイルの欠損や破損。

対処
    該当のコマンドを実行し直す。

.. _IY-O541:

IY-O541
-------

重大度: ワーニング

メッセージ::

    [IY-O541] ignisyeet.kml removed: it could not be rewritten without the flight (<k>)

意味
    飛行を除いた KML を書き直せず、``ignisyeet.kml`` を削除した。

主な原因
    出力先に書けない。

対処
    出力先の権限を確認する。

.. _IY-O542:

IY-O542
-------

重大度: ワーニング

メッセージ::

    warning[IY-O542]: resources.nice = <nice> could not be applied: <value>

意味
    ``resources.nice`` を適用できない。

主な原因
    優先度を下げる権限がない。

対処
    ``resources.nice`` を 0 にする。

.. _IY-O543:

IY-O543
-------

重大度: ワーニング

メッセージ::

    warning[IY-O543]: resources.nice is ignored on this platform

意味
    ``resources.nice`` はこのプラットフォームでは無視される。

主な原因
    Unix 以外で使っている。

対処
    ``resources.nice`` を 0 にする。

.. _IY-O544:

IY-O544
-------

重大度: ワーニング

メッセージ::

    [IY-O544] failed: sample <value> (<value>): <value>

意味
    モンテカルロのサンプルが失敗した（最大 3 件を表示）。

主な原因
    ``IY-S`` のエラーを参照する。

対処
    括弧内のエラーに従う。

