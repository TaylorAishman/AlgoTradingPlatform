# Formulas: trading-platform

Written 2026-11-22 in answer to PQ-2 of `plan.md` ("copy the papers to a stable local path to give to the agents and keep track of formulas in a formulas.md"). Needs human approval before tickets cite it.

## 1. What this document is for

Build agents work in fresh worktrees. The reference PDFs are ignored by git (R-314), so an implementer has no copy of them, and `requirements.md` cites several formulas and worked examples only by paper and page. This document states those formulas, checked against the PDFs, for the tasks named in PQ-2: C04, E02, E03, E04, H04 and H05, and for details C03, D01 and F01.

- On the owner's machine, `$ATP_REFERENCE_DIR` (default `~/atp-reference`) holds the papers: `pdf/` has the PDFs and `text/` has plain-text copies with page markers. The text copies are good for finding a page; their equations are garbled.
- **The PDFs remain the authority.** If this document and a PDF disagree, the PDF wins and this document has a defect to report.
- This document does not change `requirements.md` or `plan.md`. Where a requirement and its paper differ, or the requirement fixes something the paper leaves open, the difference is noted under the formula with the requirement ID and collected in section 12. Where neither fixes a detail that changes a tested number, the gap is an open question in section 13.

### 1.1 How it was checked

- Every equation below was read from an image of the PDF page (rendered from the PDF, not from the text copy).
- Items marked **Checked numerically** were recomputed on 2026-11-22 with short scripts (Python standard library, numpy 2.5.5, scipy 1.18.1, pandas 3.1.1). The scripts are not in the repository; each item gives its inputs and results so that it can be repeated. Section 11 lists all of them.
- Three kinds of statement are kept apart: **Paper** (transcribed), **Derived** (follows from the paper's formulas by algebra or by computation here; not printed in the paper), and **Requirement** (a choice made by `requirements.md`).

### 1.2 Citations and page numbers

Papers are cited as the other design documents cite them: short title in italics, then the PDF page index (page 1 is the first page of the file), then the equation number, for example *Deflated Sharpe Ratio* p8, eq. 2. The printed page number differs from the PDF index in some files:

| Short title | File in `$ATP_REFERENCE_DIR/pdf/` | PDF index minus printed page |
| --- | --- | --- |
| *Statistical Arbitrage* | `Statistical Arbitrage in the U.S. Equities Market.pdf` | 0 |
| *Time Series Momentum* | `Time Series Momentum.pdf` | 2 |
| *Building Diversified Portfolios* | `Building Diversified Portfolios that Outperform Out-of-Sample.pdf` | 0 |
| *Markowitz at Seventy* | `Markowitz Portfolio Construction at Seventy.pdf` | 0 |
| *Multi-Period Trading* | `Multi-Period Trading via Convex Optimization.pdf` | 3 |
| *Implementation Risk* | `Implementation Risk in Portfolio Backtesting - A Previously Unquantified Source of Error.pdf` | 0 |
| *Deflated Sharpe Ratio* | `The Deflated Sharpe Ratio - Correcting for Selection Bias, Backtest Overfitting and Non-Normality.pdf` | 0 |
| *Probability of Backtest Overfitting* | `The Probability of Backtest Overfitting.pdf` | 1 |
| *And the Cross-Section* | `And the Cross-Section of Expected Returns.pdf` | 2 |

### 1.3 Index by task

Each formula has an identifier (`SA-3`, `DSR-2`) that is never renumbered, and an anchor of the same name in lower case, so a ticket can link to `docs/design/trading-platform/formulas.md#dsr-2`.

| Task | Requirements | Formulas | Open questions |
| --- | --- | --- | --- |
| C03 eigenportfolio factors | R-300 | SA-1, SA-2, SA-3 | FQ-8 |
| C04 s-score and speed filter | R-065, R-066 | SA-4, SA-5, SA-6, SA-7, SA-9 | FQ-7 |
| (C05 band state, for context) | R-067 | SA-8 | FQ-13 |
| D01 EWMA volatility | R-089 | TSM-1, TSM-2 | FQ-9 |
| E02 HRP | R-117, R-118 | HRP-1 to HRP-6 | FQ-12 |
| E03 Markowitz, hard constraints | R-106 to R-109, R-112 | MZ-1 to MZ-6, MZ-9, MZ-10, MZ-11 | FQ-10 |
| E04 robust terms | R-110, R-111 | MZ-7, MZ-8 | FQ-11 |
| (E05 soft constraints, for context) | R-113 | MZ-12 | - |
| F01 reference backtest | R-134 | IR-1, IR-2, IR-3 | - |
| H04 PSR, DSR, effective trials | R-199, R-200, R-201 | DSR-1 to DSR-4 | FQ-1 to FQ-4 |
| H05 CSCV and PBO | R-203 to R-206 | PBO-1 to PBO-6 | FQ-5, FQ-6 |
| (H07 Table 4, beyond PQ-2) | R-208 | MT-1 | - |

## 2. C03: eigenportfolio factors (*Statistical Arbitrage*)

<a id="sa-1"></a>
### SA-1 Return matrix and standardized returns

*Statistical Arbitrage* p6 (unnumbered equations above eq. 8).

**Paper.** For $N$ stocks and a window of $M$ daily returns ending at date $t_0$ (so $M + 1$ prices), with $S_{it}$ the dividend-adjusted price of stock $i$ and $\Delta t = 1/252$:

$$R_{ik} = \frac{S_{i(t_0 - (k-1)\Delta t)} - S_{i(t_0 - k\Delta t)}}{S_{i(t_0 - k\Delta t)}}, \qquad k = 1, \dots, M, \quad i = 1, \dots, N$$

$$\bar{R}_i = \frac{1}{M} \sum_{k=1}^{M} R_{ik}, \qquad \bar{\sigma}_i^2 = \frac{1}{M-1} \sum_{k=1}^{M} (R_{ik} - \bar{R}_i)^2, \qquad Y_{ik} = \frac{R_{ik} - \bar{R}_i}{\bar{\sigma}_i}$$

Conventions:

- Simple close-to-close returns of adjusted prices. Index $k = 1$ is the most recent return; $k$ counts backwards in time.
- $\bar{\sigma}_i$ is the sample standard deviation over the same window, with divisor $M - 1$ (ddof 1). It is a daily figure; nothing is annualized.

<a id="sa-2"></a>
### SA-2 Empirical correlation matrix

*Statistical Arbitrage* p6, eq. 8.

**Paper.**

$$\rho_{ij} = \frac{1}{M-1} \sum_{k=1}^{M} Y_{ik} Y_{jk}$$

It is symmetric, non-negative definite, and $\rho_{ii} = 1$. Its eigenvalues are ranked in decreasing order, $N \ge \lambda_1 > \lambda_2 \ge \dots \ge \lambda_N \ge 0$, with eigenvectors $v^{(j)} = (v^{(j)}_1, \dots, v^{(j)}_N)$ (p7).

- **Derived.** $\rho$ equals the ordinary Pearson correlation matrix of the $M \times N$ return window (`numpy.corrcoef`). **Checked numerically.**
- The paper does not state the eigenvector normalization. Unit length is assumed here (what `numpy.linalg.eigh` returns). The sign of an eigenvector is arbitrary; flipping it flips the sign of the factor return of SA-3 and leaves the regression residuals of SA-4 unchanged.
- With $N = 500$ and $M = 252$ the matrix has rank at most $M - 1$; the trailing eigenvalues are zero. Only the leading ones are used.

<a id="sa-3"></a>
### SA-3 Eigenportfolio weights and factor returns

*Statistical Arbitrage* p9, eq. 9 (weights in the unnumbered equation above it).

**Paper.** For each retained eigenvector $j = 1, \dots, m$, the eigenportfolio invests $Q^{(j)}_i$ in stock $i$, and its return on day $k$ is the factor return $F_{jk}$:

$$Q^{(j)}_i = \frac{v^{(j)}_i}{\bar{\sigma}_i}, \qquad F_{jk} = \sum_{i=1}^{N} \frac{v^{(j)}_i}{\bar{\sigma}_i} R_{ik}, \qquad j = 1, \dots, m$$

The paper states that the eigenportfolio returns are uncorrelated: the empirical correlation of $F_j$ and $F_{j'}$ vanishes for $j \ne j'$.

- The returns in eq. 9 are the raw $R_{ik}$, not the demeaned ones.
- The paper offers two ways to choose $m$ (p8): a fixed number, or as many as explain a given share of the trace. It reports about 15 factors (p6).
- **Requirement.** R-300 fixes the window at 252 sessions and $m$ at 15, both configurable, and takes the fixed-number variant; the variable-number variant is out of scope (requirements section 6).
- **Derived, and it matters for R-300's test.** The factors are uncorrelated over the window only when the "volatility" in the denominator is the window's own sample standard deviation $\bar{\sigma}_i$ of SA-1. Then the sample variance of $F_j$ over the window (ddof 1) equals $\lambda_j$. **Checked numerically:** on a random 252 by 40 panel with 5 factors the largest off-diagonal correlation was below 2e-15 and the variances matched the eigenvalues; a uniform change of divisor (ddof 0 for every stock) keeps the result; dividing by a volatility from another estimator (each $\bar{\sigma}_i$ scaled by an independent factor between 0.7 and 1.3, as an EWMA estimate would be) gave off-diagonal correlations up to 0.08. The volatility estimator of R-089 must therefore not be used here.
- R-300 says "trailing 252 sessions" without saying whether that is 252 returns (253 prices), as $M$ is in the paper, or 252 prices (251 returns). See FQ-8.

## 3. C04: residual process and s-score (*Statistical Arbitrage*)

<a id="sa-4"></a>
### SA-4 Residual regression and cumulative residual

*Statistical Arbitrage* p45 (appendix, unnumbered equations). The model behind it is eq. 10 on p14 and eq. 12 on p16.

**Paper.** The appendix describes the sector-ETF case and says the PCA case is similar. Over the 60 most recent daily returns, in chronological order ($n = 60$ is the last observed return):

$$R^{S}_n = \beta_0 + \beta R^{I}_n + \epsilon_n, \qquad n = 1, 2, \dots, 60$$

$$X_k = \sum_{j=1}^{k} \epsilon_j, \qquad k = 1, 2, \dots, 60$$

$R^S_n$ is the stock return and $R^I_n$ the factor return. The regression has an intercept $\beta_0$; the paper sets the drift $\alpha = \beta_0 / \Delta t = 252 \beta_0$ (not used by the pure mean-reversion signal). Because the residuals of a regression with an intercept sum to zero, $X_{60} = 0$.

- **Derived (PCA case, R-065).** The regressors are the $m$ factor returns of SA-3 on the same 60 sessions: $R^{S}_n = \beta_0 + \sum_{j=1}^{m} \beta_j F_{jn} + \epsilon_n$. With $m = 15$ this is 16 coefficients from 60 observations.
- Note the two index directions: SA-1 counts $k$ backwards from the most recent return, the appendix counts $n$ forwards in time.
- R-065 does not mention the intercept. Without it $X_{60} \ne 0$ and eq. 22 no longer applies as written. See FQ-7.
- **Checked numerically:** with an intercept, $X_{60}$ was zero to 4e-16 for every stock of a 40-stock example.

<a id="sa-5"></a>
### SA-5 Ornstein-Uhlenbeck parameters

*Statistical Arbitrage* p45-46, eq. 21 (model: p16, eq. 12-13; equilibrium variance: p17, eq. 14).

**Paper.** The residual process is modelled as $dX_i(t) = \kappa_i (m_i - X_i(t)) dt + \sigma_i dW_i(t)$ with $\kappa_i > 0$ (eq. 12). It is estimated by the one-lag regression on the 60 values of SA-4, which has 59 observations:

$$X_{n+1} = a + b X_n + \zeta_{n+1}, \qquad n = 1, \dots, 59$$

$$a = m (1 - e^{-\kappa \Delta t}), \qquad b = e^{-\kappa \Delta t}, \qquad \mathrm{Variance}(\zeta) = \sigma^2 \frac{1 - e^{-2\kappa \Delta t}}{2\kappa}$$

whence (eq. 21):

$$\kappa = -\ln(b) \cdot 252, \qquad m = \frac{a}{1 - b}, \qquad \sigma = \sqrt{\frac{\mathrm{Variance}(\zeta) \cdot 2\kappa}{1 - b^2}}, \qquad \sigma_{eq} = \sqrt{\frac{\mathrm{Variance}(\zeta)}{1 - b^2}}$$

Conventions:

- $\Delta t = 1/252$, so $\kappa$ and $\sigma$ are annual and $m$, $\sigma_{eq}$ are in units of cumulative daily return. The logarithm is natural.
- $a$ and $b$ are the ordinary least-squares intercept and slope.
- The formulas need $0 < b < 1$. For $b \le 0$ the logarithm is undefined; for $b \ge 1$ there is no mean reversion.
- The paper does not say which divisor $\mathrm{Variance}(\zeta)$ uses (59, 58 or 57). See FQ-7; the choice moves the s-score in the second significant digit.

<a id="sa-6"></a>
### SA-6 Mean-reversion speed filter

*Statistical Arbitrage* p19 (section 4) and p46 (below eq. 21). The characteristic time is $\tau_i = 1/\kappa_i$ (p17).

**Paper.** With an estimation window of $T_1 = 60/252$ years, stocks are kept when the mean-reversion time is less than half the window:

$$\kappa > \frac{252}{30} = 8.4 \quad \Longleftrightarrow \quad 0 < b < 0.9672$$

If $b$ is too close to 1 the model is rejected for that stock.

- **Derived.** The bound on $b$ is $e^{-1/30} = 0.967216$; the paper prints 0.9672. **Checked numerically.**
- **Requirement.** R-066 returns NaN when the estimated speed is below 252/30 per year. It does not name the case $b \le 0$, where $\kappa$ is undefined (FQ-7).

<a id="sa-7"></a>
### SA-7 s-score

*Statistical Arbitrage* p19, eq. 15 (definition); p46, eq. 22 (estimate with the centred mean).

**Paper.** The definition, with $\sigma_{eq,i} = \sigma_i / \sqrt{2\kappa_i}$:

$$s_i = \frac{X_i(t) - m_i}{\sigma_{eq,i}}$$

Since $X(t) = X_{60} = 0$, this becomes

$$s = \frac{-m}{\sigma_{eq}} = \frac{-a \sqrt{1 - b^2}}{(1 - b) \sqrt{\mathrm{Variance}(\zeta)}}$$

The paper then centres the mean across stocks, with $\langle \cdot \rangle$ denoting the average over different stocks:

$$\bar{m} = \frac{a}{1 - b} - \left\langle \frac{a}{1 - b} \right\rangle$$

and the s-score used is (eq. 22):

$$s = \frac{-\bar{m}}{\sigma_{eq}} = \frac{-a \sqrt{1 - b^2}}{(1 - b) \sqrt{\mathrm{Variance}(\zeta)}} + \left\langle \frac{a}{1 - b} \right\rangle \sqrt{\frac{1 - b^2}{\mathrm{Variance}(\zeta)}}$$

- Sign: a positive s-score means the cumulative residual is above its equilibrium (the stock is rich against its factors).
- $a$, $b$ and $\mathrm{Variance}(\zeta)$ are per stock; only the bracket is a cross-sectional quantity.
- **Requirement.** R-065 asks for eq. 21-22 with the mean centred across assets. That matches the paper.
- The paper does not say which stocks enter the average: all stocks of the universe, or only those that pass SA-6. See FQ-7.
- **Checked numerically:** the two sides of eq. 22 agree for every stock of a 40-stock example.

<a id="sa-8"></a>
### SA-8 Entry and exit bands (context for C05)

*Statistical Arbitrage* p20, eq. 16.

**Paper.** Buy to open if $s_i < -\bar{s}_{bo}$; sell to open if $s_i > +\bar{s}_{so}$; close a short position if $s_i < +\bar{s}_{bc}$; close a long position if $s_i > -\bar{s}_{sc}$, with

$$\bar{s}_{bo} = \bar{s}_{so} = 1.25, \qquad \bar{s}_{bc} = 0.75, \qquad \bar{s}_{sc} = 0.50$$

- The paper's summary on p4 gives one exit level, 0.5; eq. 16 on p20 closes short positions at 0.75 and long positions at 0.50.
- **Requirement.** R-067 opens when the absolute s-score exceeds 1.25 and closes when it falls below 0.5, for both sides. It follows p4, not eq. 16. See FQ-13.
- The direction is contrarian: a long position is opened at a negative s-score, which is why R-067 returns minus the s-score.

<a id="sa-9"></a>
### SA-9 Worked values for the s-score

**The paper contains no numeric worked example of the s-score.** The appendix gives formulas only; Table 3 (p19) and the figures give descriptive statistics, not inputs. R-065's acceptance check speaks of a "fixture transcribed from the PDF": there is nothing numeric to transcribe, so the fixture has to be computed from SA-4 to SA-7 with the conventions of FQ-7 fixed.

**Derived.** A check of the closed forms alone, for one stock with $a = 0.002$, $b = 0.8$, $\mathrm{Variance}(\zeta) = 0.0001$ and no centring:

| Quantity | Value |
| --- | --- |
| $\kappa = -\ln(0.8) \cdot 252$ | 56.232175 |
| $m = a / (1 - b)$ | 0.01 |
| $\sigma_{eq} = \sqrt{0.0001 / 0.36}$ | 0.016667 |
| $\sigma$ | 0.176749 |
| $s = -m / \sigma_{eq}$ | -0.6 |

**Checked numerically.** On a 60-session example the choice of divisor for $\mathrm{Variance}(\zeta)$ alone moved one stock's centred s-score between 1.2654 (divisor 59), 1.2546 (58) and 1.2437 (57). R-065's tolerance is 1e-9, so the fixture and the implementation must share one convention.

## 4. D01: EWMA volatility (*Time Series Momentum*)

<a id="tsm-1"></a>
### TSM-1 Exponentially weighted variance

*Time Series Momentum* p13, eq. 1 (the lag rule continues on p14).

**Paper.** The ex-ante annualized variance of an instrument is

$$\sigma_t^2 = 261 \sum_{i=0}^{\infty} (1 - \delta) \delta^i (r_{t-1-i} - \bar{r}_t)^2$$

where the weights $(1 - \delta) \delta^i$ add up to 1, $\bar{r}_t$ is the exponentially weighted average return computed in the same way, and $\delta$ is chosen so that the centre of mass of the weights is 60 days:

$$\sum_{i=0}^{\infty} (1 - \delta) \delta^i i = \frac{\delta}{1 - \delta} = 60 \quad \Longrightarrow \quad \delta = \frac{60}{61}$$

Conventions:

- $r$ are daily returns. The most recent return has weight $(1 - \delta)$, the one before it $(1 - \delta)\delta$, and so on.
- Lag: $\sigma_t$ uses returns up to $t - 1$ only. The paper applies the estimate of time $t - 1$ to the return of time $t$ (p14).
- The mean subtracted is the exponentially weighted mean, not the simple mean and not zero.
- The scalar 261 annualizes the variance.
- **Requirement.** R-089 takes the centre of mass (decay 60/61) and "returns up to the last session visible in the view", which is the paper's lag seen from the decision instant: the estimate made after the close of session $T$ is the one used for session $T + 1$.
- **Requirement differs or is silent** on three points, each of which changes the number by far more than R-089's tolerance of 1e-12 (FQ-9):
  1. Annualization. The paper multiplies by 261. `requirements.md` uses 252 periods per year elsewhere (R-198), and R-089 does not say whether the estimator returns a per-session or an annual figure. The cost model's `sigma` (R-095) and the risk estimate are per session.
  2. Finite history. The paper's sum is infinite. Over $n$ returns the weights sum to $1 - \delta^n$ (0.9845 for $n = 252$), so the estimate depends on whether the weights are renormalized.
  3. The mean. R-089 says "exponentially weighted variance" without naming the mean.
- **Checked numerically.** The centre of mass at $\delta = 60/61$ is 60 and the weights sum to 1. On one 400-return series: renormalized weights with the exponentially weighted mean gave a variance of 1.36628e-4, identical to pandas `ewm(com=60, adjust=True).var(bias=True)`; pandas' default `var()` (bias-corrected) gave 1.37770e-4; `ewm(com=60, adjust=False).var(bias=True)` gave 1.36555e-4; the literal truncated sum of eq. 1 gave 1.36444e-4.

<a id="tsm-2"></a>
### TSM-2 Position sizing by ex-ante volatility (context for C01)

*Time Series Momentum* p18, eq. 5.

**Paper.** The return of the 12-month time-series momentum strategy for instrument $s$:

$$r^{TSMOM,s}_{t,t+1} = \mathrm{sign}(r^{s}_{t-12,t}) \frac{40\%}{\sigma^{s}_{t}} r^{s}_{t,t+1}$$

- $r^{s}_{t-12,t}$ is the excess return over the past 12 months, $\sigma^{s}_{t}$ the annualized ex-ante volatility of TSM-1, and 40% an arbitrary annual volatility target.
- **Requirement.** R-063 and R-064 take the sign of the trailing 252-session excess return and a score inversely proportional to the R-089 volatility. The constant (40% here) is not part of the requirements, and R-064's test (doubling volatility halves the score) does not depend on it or on the annualization factor.

## 5. E02: Hierarchical Risk Parity (*Building Diversified Portfolios*)

The algorithm has three stages (p5): tree clustering, quasi-diagonalization, recursive bisection. Inputs are the covariance matrix $V$ of the assets and the correlation matrix $\rho$ derived from it. No matrix is inverted.

<a id="hrp-1"></a>
### HRP-1 Correlation distance and distance of distances

*Building Diversified Portfolios* p5 (stage 1, first and second steps; proof that $d$ is a metric on p13).

**Paper.**

$$d_{i,j} = \sqrt{\tfrac{1}{2} (1 - \rho_{i,j})}, \qquad 0 \le d_{i,j} \le 1$$

$$\tilde{d}_{i,j} = \sqrt{\sum_{n=1}^{N} (d_{n,i} - d_{n,j})^2}, \qquad 0 \le \tilde{d}_{i,j} \le \sqrt{N}$$

$D = (d_{i,j})$ is the $N \times N$ distance matrix, and $\tilde{d}_{i,j}$ is the Euclidean distance between columns $i$ and $j$ of $D$.

- The clustering of HRP-2 runs on $\tilde{d}$, not on $d$. The paper's own code passes the square matrix $D$ to scipy's `linkage`, which treats its rows as observations and so computes $\tilde{d}$ itself. Passing $d$ in condensed form instead is a different algorithm. **Checked numerically** on the paper's example: the two give different merge distances (0.5659 and 0.9747 against 0.3873 and 0.6325).

<a id="hrp-2"></a>
### HRP-2 Tree clustering (single linkage)

*Building Diversified Portfolios* p6 (third to sixth steps) and p7 (linkage matrix).

**Paper.**

1. Join the pair of columns with the smallest distance: $(i^{\ast}, j^{\ast}) = \mathrm{argmin}_{i \ne j} \tilde{d}_{i,j}$. Call the cluster $u[1]$.
2. Define the distance from every other item $i$ to the new cluster by the nearest point ("single linkage"): $\dot{d}_{i,u[1]} = \min_{j \in u[1]} \tilde{d}_{i,j}$.
3. Append that row and column to the matrix and drop the rows and columns of the clustered items.
4. Repeat until $N - 1$ clusters have been formed and the last one contains every item.

The result is a linkage matrix with one row $(y_{m,1}, y_{m,2}, y_{m,3}, y_{m,4})$ per cluster $m = 1, \dots, N - 1$: the two constituents, the distance between them, and the number of original items in the cluster. (The paper's sentence calls the count $y_{m,3}$ a second time; it is the fourth entry, as its code reads it.)

- The paper does not say how ties are broken. In the diagonal case of HRP-5 every distance is equal and the result does not depend on the order.

<a id="hrp-3"></a>
### HRP-3 Quasi-diagonalization

*Building Diversified Portfolios* p7 (stage 2).

**Paper.** Start from the two constituents of the last row of the linkage matrix, $(y_{N-1,1}, y_{N-1,2})$. Replace every cluster in the list by its two constituents, in place and in order, until only original items remain. The output is an ordering of the $N$ assets in which similar assets are adjacent. The covariance matrix is only reordered; there is no change of basis.

<a id="hrp-4"></a>
### HRP-4 Recursive bisection

*Building Diversified Portfolios* p8 (stage 3).

**Paper.**

1. Start with one list $L_0$ holding all items in the order of HRP-3, and weights $w_n = 1$ for all $n$.
2. Stop when every list has one item.
3. For each list $L_i$ with more than one item:
   1. Bisect it into $L_i^{(1)}$ and $L_i^{(2)}$, preserving the order, where the first part has $\mathrm{int}[\tfrac{1}{2} \lvert L_i \rvert]$ items (the first part is the smaller one when the count is odd).
   2. For each part $j = 1, 2$, with $V_i^{(j)}$ the covariance matrix of its constituents, compute the variance of its inverse-variance portfolio (formulas below).
   3. Compute the split factor $\alpha_i$ (below).
   4. Multiply $w_n$ by $\alpha_i$ for every $n$ in $L_i^{(1)}$.
   5. Multiply $w_n$ by $1 - \alpha_i$ for every $n$ in $L_i^{(2)}$.
4. Go to step 2.

$$\tilde{w}_i^{(j)} = \frac{\mathrm{diag}[V_i^{(j)}]^{-1}}{\mathrm{tr}[\mathrm{diag}[V_i^{(j)}]^{-1}]}, \qquad \tilde{V}_i^{(j)} = \tilde{w}_i^{(j)\top} V_i^{(j)} \tilde{w}_i^{(j)}, \qquad \alpha_i = 1 - \frac{\tilde{V}_i^{(1)}}{\tilde{V}_i^{(1)} + \tilde{V}_i^{(2)}}$$

The paper states that the algorithm guarantees $0 \le w_i \le 1$ for all $i$ and $\sum_i w_i = 1$.

- $\mathrm{diag}[V]^{-1}$ is the vector of reciprocal variances; $\tilde{w}$ is the inverse-variance portfolio of the part, and $\tilde{V}$ uses the part's full covariance block, off-diagonal entries included.
- The weights are inverse-variance, not inverse-volatility. The inverse-volatility allocator of E01 (R-119) is a different rule.
- **Requirement.** R-117 asks for non-negative weights summing to 1, including for a singular covariance matrix. Only the diagonal of $V$ is ever inverted, so a singular matrix is no obstacle; a zero on the diagonal is (FQ-12).
- **Checked numerically:** for 50 assets estimated from 30 sessions (rank 29) the weights were all positive (smallest 0.0051) and summed to 1.

<a id="hrp-5"></a>
### HRP-5 The diagonal case

*Building Diversified Portfolios* p13-14 (appendix A.2).

**Paper.** The minimum-variance portfolio for a characteristic vector $a$ is $\omega = V^{-1} a / (a^{\top} V^{-1} a)$. For a diagonal $V$ and $a = 1_N$:

$$\omega_n = \frac{V_{n,n}^{-1}}{\sum_{i=1}^{N} V_{i,i}^{-1}}$$

and for $N = 2$, $\omega_1 = 1 - V_{1,1} / (V_{1,1} + V_{2,2})$, which is the split of HRP-4.

- **Derived (R-118).** For a diagonal covariance matrix, the cluster variance of HRP-4 is $\tilde{V} = 1 / \sum_{n} V_{n,n}^{-1}$ over the part, so each split hands a part its share of the total reciprocal variance, and the final weights are exactly the inverse-variance weights above, whatever order HRP-3 produced. **Checked numerically:** 11 assets with random variances, largest difference from the inverse-variance weights 4e-17.

<a id="hrp-6"></a>
### HRP-6 Worked examples

*Building Diversified Portfolios* p5-6, Examples 1 to 6.

**Paper.** For the correlation matrix with $\rho_{1,2} = 0.7$, $\rho_{1,3} = 0.2$, $\rho_{2,3} = -0.2$:

| Quantity | (1,2) | (1,3) | (2,3) |
| --- | --- | --- | --- |
| $d_{i,j}$ | 0.3873 | 0.6325 | 0.7746 |
| $\tilde{d}_{i,j}$ | 0.5659 | 0.9747 | 1.1225 |

The first cluster is $u[1] = (1, 2)$ at distance 0.5659. Its distance to item 3 is $\min[0.9747, 1.1225] = 0.9747$, so the second cluster is $u[2] = (3, 4)$, where 4 denotes $u[1]$, at distance 0.9747. **Checked numerically** (scipy single linkage on the Euclidean distances between the rows of $D$ reproduces both merges and both distances).

**Derived** (the paper stops at the clustering). Giving the three assets unit variances, so that the covariance matrix equals this correlation matrix: the order from HRP-3 is (3, 1, 2); the first bisection is {3} against {1, 2}; $\tilde{V}^{(1)} = 1$ and $\tilde{V}^{(2)} = \tfrac{1}{4}(1 + 1 + 2 \cdot 0.7) = 0.85$; $\alpha = 1 - 1/1.85 = 0.459459$; the second bisection splits {1, 2} evenly. The weights are $w_1 = w_2 = 0.270270$ and $w_3 = 0.459459$. **Checked numerically.**

The paper's Exhibit 7 (p25) compares allocations on simulated data generated by the Python 2 code of its appendix with a fixed seed. It was not regenerated here and is not used as a fixture.

## 6. E03 and E04: the Markowitz problem (*Markowitz at Seventy*, *Multi-Period Trading*)

All quantities are fractions of the total portfolio value $V$ for one trading period, unless a line says otherwise. The mapping to the interface of Arch 5.6 is in MZ-9.

<a id="mz-1"></a>
### MZ-1 Weights, cash and trades

*Markowitz at Seventy* p13, eq. 3; p15, eq. 6; p16, eq. 7.

**Paper.** With $w \in \mathbb{R}^n$ the asset weights (negative for short positions), $c$ the cash weight, and $w^{pre}$, $c^{pre}$ the pre-trade weights:

$$1^{\top} w + c = 1, \qquad z = w - w^{pre}, \qquad c = c^{pre} - 1^{\top} z$$

- $z_i = 0.01$ means buying asset $i$ for 1% of the portfolio value.
- The post-trade cash of eq. 7 does not include holding or transaction costs.
- **Requirement.** R-105: weights plus `cash_weight` sum to 1 to 1e-8.

<a id="mz-2"></a>
### MZ-2 Leverage and weight limits

*Markowitz at Seventy* p14 (unnumbered) and p15, eq. 4.

**Paper.**

$$L = \sum_{i=1}^{n} \lvert w_i \rvert = \lVert w \rVert_1, \qquad L \le L^{tar}$$

$$w^{min} \le w \le w^{max}, \qquad c^{min} \le c \le c^{max}$$

- $L$ is gross leverage and excludes cash. A long-only portfolio has $w^{min} = 0$.

<a id="mz-3"></a>
### MZ-3 Holding cost

*Markowitz at Seventy* p15, eq. 5.

**Paper.**

$$\phi^{hold}(w, c) = (\kappa^{short})^{\top} (-w)_{+} + \kappa^{borrow} (-c)_{+}$$

with $(a)_{+} = \max(a, 0)$ elementwise, $\kappa^{short} \ge 0$ the vector of borrow rates of the assets and $\kappa^{borrow} \ge 0$ the borrow rate of cash.

- Rates are per period. **Requirement.** R-101 states the same in dollars (`short_fee`, `cash_borrow_rate`, per-session rates).

<a id="mz-4"></a>
### MZ-4 Turnover and participation

*Markowitz at Seventy* p16 (turnover, unnumbered), eq. 8 and eq. 9.

**Paper.**

$$T = \frac{1}{2} \sum_{i=1}^{n} \lvert z_i \rvert = \frac{1}{2} \lVert z \rVert_1, \qquad T \le T^{tar}$$

$$z^{min} \le z \le z^{max}, \qquad \text{for example} \quad \lvert z \rvert \le 0.05 v$$

where $v \in \mathbb{R}^n$ is the trading volume of each asset expressed as a multiple of the portfolio value, and a forecast of it is used because the realized volume is unknown when $z$ is chosen.

- **Requirement.** R-107 has the same turnover. Its participation limit is in dollars: the absolute dollar trade of each asset is at most `participation_max` times its forecast `adv_dollars`. Dividing both sides by `nav` gives eq. 8 with $v_i$ equal to `adv_dollars` over `nav` and `participation_max` in place of the paper's example value 0.05.
- **Requirement.** R-108 (restricted assets are not traded) is the trade limit $z^{min}_i = z^{max}_i = 0$.

<a id="mz-5"></a>
### MZ-5 Trading cost

*Markowitz at Seventy* p17, eq. 10. *Multi-Period Trading* p12, eq. 2.2 and p13, eq. 2.3.

**Paper (*Markowitz at Seventy*).** As a fraction of portfolio value:

$$\phi^{trade}(z) = (\kappa^{spread})^{\top} \lvert z \rvert + (\kappa^{impact})^{\top} \lvert z \rvert^{3/2}, \qquad \kappa^{impact}_i = a s_i v_i^{-1/2}$$

$\kappa^{spread}$ is one half of the bid-ask spread of each asset, $s_i$ the volatility of asset $i$ over the trading period, $v_i$ the market volume as a multiple of the portfolio value, and $a$ a constant of order one. Powers and absolute values are elementwise.

**Paper (*Multi-Period Trading*).** For one asset and a dollar trade $x$ (eq. 2.2), and for the normalized trade $z_i = x_i / v$ with $v$ the portfolio value (eq. 2.3):

$$x \mapsto a \lvert x \rvert + b \sigma \frac{\lvert x \rvert^{3/2}}{V^{1/2}} + c x$$

$$a_i \lvert z_i \rvert + b_i \sigma_i \frac{\lvert z_i \rvert^{3/2}}{(V_i / v)^{1/2}} + c_i z_i$$

Here $a$ is one half of the bid-ask spread as a fraction of the price, $V$ the market volume of the asset in dollars for the period, $\sigma$ the price volatility over recent periods, $b$ a constant of about one, and $c$ a term that makes buying and selling cost differently.

- Beware the letters: in *Multi-Period Trading* $a$ is the half-spread and $b$ the impact constant; in *Markowitz at Seventy* $a$ is the impact constant.
- **Derived.** Eq. 2.3 times the portfolio value equals eq. 2.2 at $x = z v$, which is the identity R-099 tests. **Checked numerically.**
- **Requirement.** R-095 is eq. 2.2 with $c = 0$ (R-096 requires a symmetric cost), `half_spread` for $a$, `impact_coeff` for $b$, `sigma` the per-session return volatility as a fraction, and `adv_dollars` for $V$. *Multi-Period Trading* describes $\sigma$ as a price volatility in dollars with $b$ in inverse dollars; the requirement's unitless form is the one of *Markowitz at Seventy* eq. 10.

<a id="mz-6"></a>
### MZ-6 Expected return, risk and the risk target

*Markowitz at Seventy* p19 (unnumbered) and eq. 13.

**Paper.** With $\mu$ the forecast mean and $\Sigma$ the forecast covariance of the asset returns over the period, and $r^{rf}$ the risk-free rate:

$$\bar{R} = \mu^{\top} w + r^{rf} c, \qquad \sigma^2 = w^{\top} \Sigma w, \qquad \sigma \le \sigma^{tar}$$

- Risk is the standard deviation $\sigma$, for the same period as $\Sigma$. The paper annualizes a risk by multiplying it by the square root of the number of periods per year.
- With a factor model (p20), $\Sigma = F \Sigma^{f} F^{\top} + D$ and $\sigma^2 = (F^{\top} w)^{\top} \Sigma^{f} (F^{\top} w) + w^{\top} D w$.
- `Constraints.risk_target_annual` is an annual figure while the risk estimate is per session; neither document states the conversion (FQ-10).

<a id="mz-7"></a>
### MZ-7 Robust return (E04)

*Markowitz at Seventy* p22-23, eq. 16.

**Paper.** With $\rho \in \mathbb{R}^n_{+}$ the half-width of the uncertainty interval of each mean return, the worst-case mean portfolio return is

$$R^{wc} = \min \lbrace (\mu + \delta)^{\top} w : \lvert \delta \rvert \le \rho \rbrace = \bar{R} - \rho^{\top} \lvert w \rvert$$

- The adversary picks $\mu_i - \rho_i$ for a long position and $\mu_i + \rho_i$ for a short one.
- **Requirement.** R-110 writes the return term as `mu @ w - rho_scale * rho @ abs(w)`. Two differences from eq. 16: the term $r^{rf} c$ of $\bar{R}$ is absent (the cash rate is zero by default, RQ6), and `rho_scale` is a multiplier the paper does not have (it is a field of `AllocationInput`, Arch 5.6). With `rho_scale` equal to 1 and a zero cash rate the two agree.
- **Derived.** When $\rho_i > \lvert \mu_i \rvert$ for every asset, $\mu_i w_i - \rho_i \lvert w_i \rvert < 0$ for every non-zero $w_i$, so the return term is maximized by $w = 0$ (all cash). This is R-110's test; it needs constraints that allow $w = 0$. **Checked numerically** (20,000 random weight vectors, all with a negative value).

<a id="mz-8"></a>
### MZ-8 Worst-case risk (E04)

*Markowitz at Seventy* p23 (uncertainty set); p24, eq. 17 and eq. 18.

**Paper.** With perturbations $\Delta$ of the covariance bounded by $\lvert \Delta_{ij} \rvert \le \varrho (\Sigma_{ii} \Sigma_{jj})^{1/2}$ and $\varrho \in [0, 1)$:

$$(\sigma^{wc})^2 = \max \lbrace w^{\top} (\Sigma + \Delta) w : \lvert \Delta_{ij} \rvert \le \varrho (\Sigma_{ii} \Sigma_{jj})^{1/2} \rbrace = \sigma^2 + \varrho \left( \sum_{i=1}^{n} \Sigma_{ii}^{1/2} \lvert w_i \rvert \right)^2$$

$$\sigma^{wc} = \left\lVert \left( \sigma, \; \sqrt{\varrho} \, (\mathrm{diag}(\Sigma)^{1/2})^{\top} \lvert w \rvert \right) \right\rVert_2$$

For a long-only portfolio the worst-case risk is the risk under the covariance matrix $\Sigma + \varrho s s^{\top}$, with $s = \mathrm{diag}(\Sigma)^{1/2}$ the vector of asset volatilities.

- Eq. 17 gives the square of $\sigma^{wc}$; eq. 18 is the same quantity as a Euclidean norm, the form a cone solver takes.
- $\Sigma_{ii}$ is the total variance of asset $i$. With a factor model it is the diagonal of $F \Sigma^{f} F^{\top} + D$, not $D$ alone.
- **Requirement.** R-111: `cov_uncertainty` is $\varrho$, and the `sigma_wc` diagnostic must equal "that formula" at the solution to 1e-8. Whether the diagnostic is $\sigma^{wc}$ or its square, and per session or annual, is not stated (FQ-11).
- **Checked numerically** on a random 4-asset case with long and short weights and $\varrho = 0.04$: the maximum over the uncertainty set is reached at $\Delta_{ij} = \varrho (\Sigma_{ii} \Sigma_{jj})^{1/2} \, \mathrm{sign}(w_i w_j)$ and equals eq. 17; eq. 18 equals the square root of eq. 17; for long-only weights eq. 17 equals the risk under $\Sigma + \varrho s s^{\top}$.

<a id="mz-9"></a>
### MZ-9 The Markowitz problem (19)

*Markowitz at Seventy* p24, eq. 19.

**Paper.**

$$\text{maximize} \quad R^{wc} - \gamma^{hold} \phi^{hold}(w, c) - \gamma^{trade} \phi^{trade}(z)$$

subject to

$$1^{\top} w + c = 1, \qquad z = w - w^{pre}$$

$$w^{min} \le w \le w^{max}, \qquad L \le L^{tar}, \qquad c^{min} \le c \le c^{max}$$

$$z^{min} \le z \le z^{max}, \qquad T \le T^{tar}$$

$$\sigma^{wc} \le \sigma^{tar}$$

with variables $w \in \mathbb{R}^n$ and $c \in \mathbb{R}$, and positive parameters $\gamma^{hold}$ and $\gamma^{trade}$. The paper states that the problem is convex.

Mapping to Arch 5.6 (the right-hand column is this document's reading of the interface, to be checked against A02's types):

| Problem (19) | `AllocationInput` / `Constraints` / `CostParams` |
| --- | --- |
| $w^{pre}$, $c^{pre}$ | `w_pre`, `cash_pre` |
| $\mu$, $\rho$ | `forecast.mu`, `forecast.rho` (times `rho_scale`, MZ-7) |
| $\Sigma$ or $F$, $\Sigma^{f}$, $D$ | `risk.cov` or `risk.factor_loadings`, `risk.factor_cov`, `risk.idio_var` |
| $\varrho$ | `cov_uncertainty` |
| $\gamma^{trade}$, $\gamma^{hold}$ | `gamma_trade`, `gamma_hold` |
| $\phi^{trade}$ | `CostModel.trade_cost_expr(z, nav, cost)` (MZ-5, R-099) |
| $\kappa^{short}$, $\kappa^{borrow}$ | `cost.short_fee`, `cost.cash_borrow_rate` |
| $w^{min}$, $w^{max}$ | `w_min`, `w_max`; `long_only` sets the lower limit to 0 |
| $c^{min}$, $c^{max}$ | `cash_min`, `cash_max` |
| $L^{tar}$, $T^{tar}$ | `leverage_max`, `turnover_max` |
| $z^{min}$, $z^{max}$ | `participation_max` (MZ-4) and `restricted` (R-108) |
| $\sigma^{tar}$ | `risk_target_annual` (units: FQ-10) |

- **Requirement.** E03 builds (19) with $\varrho = 0$ and $\rho = 0$ in effect; E04 adds MZ-7 and MZ-8. R-106 lists the hard constraints: long-only, weight bounds, cash bounds, leverage, turnover, participation and risk target, each to 1e-6.
- The paper takes $\Sigma$ positive definite. R-090 allows a smallest eigenvalue slightly below zero, so the risk term needs a factorization that tolerates it.

<a id="mz-10"></a>
### MZ-10 Trade-aversion factor

*Multi-Period Trading* p44 (section 4.8).

**Paper.** The whole transaction cost function is scaled by a trading-aversion factor $\gamma^{trade}$. Increasing it deters trading and reduces turnover; decreasing it increases both. $1/\gamma^{trade}$ can be read as the number of periods over which a transaction cost is amortized. The holding cost is scaled by $\gamma^{hold}$ in the same way.

- **Requirement.** R-112: raising `gamma_trade` with everything else fixed must never raise turnover (five values, tolerance 1e-6). The paper states the direction, not a proof for every problem instance.

<a id="mz-11"></a>
### MZ-11 Analytic two-asset case (R-109)

Not printed in the paper. **Derived** from MZ-6 and MZ-9.

With no costs, $\rho = 0$, $\varrho = 0$, a zero cash rate and only the risk target binding, problem (19) is: maximize $\mu^{\top} w$ subject to $w^{\top} \Sigma w \le (\sigma^{tar})^2$. Setting the gradient of the Lagrangian to zero gives $w$ proportional to $\Sigma^{-1} \mu$, and the constraint fixes the scale:

$$w^{\star} = \sigma^{tar} \frac{\Sigma^{-1} \mu}{\sqrt{\mu^{\top} \Sigma^{-1} \mu}}, \qquad c^{\star} = 1 - 1^{\top} w^{\star}$$

- This is R-109's formula. It holds when $\mu \ne 0$ and the weight, cash, leverage, turnover and participation limits are all slack at $w^{\star}$; the test must choose inputs for which they are.
- $\sigma^{tar}$ and $\Sigma$ must be in the same time units (FQ-10).
- **Checked numerically** against a general-purpose constrained optimizer (scipy SLSQP): for $\Sigma = \tfrac{1}{252} [[0.04, 0.012], [0.012, 0.09]]$, $\mu = (0.0006, 0.0004)$ and $\sigma^{tar} = 0.10 / \sqrt{252}$, both give $w^{\star} = (0.460424, 0.082352)$ and $c^{\star} = 0.457223$.

<a id="mz-12"></a>
### MZ-12 Soft-constraint penalty (context for E05)

*Markowitz at Seventy* p26.

**Paper.** A hard constraint $f \le f^{max}$ is softened by removing it and subtracting from the objective the penalty

$$\gamma (f - f^{max})_{+}$$

where $(f - f^{max})_{+}$ is the violation and $\gamma > 0$ the priority. For the risk limit the penalty is $\gamma^{risk} (\sigma - \sigma^{tar})_{+}$. The paper keeps hard: the first line of constraints of (19), the long-only constraint, and a no-borrowing constraint $c \ge 0$. It notes that the softened problem is always feasible because $z = 0$ is.

- **Requirement.** R-113 and R-114 say the same (budget, long-only and minimum cash cannot be soft).

## 7. F01: reference backtest (*Implementation Risk*)

<a id="ir-1"></a>
### IR-1 Algorithm 1, proportional-cost backtest loop

*Implementation Risk* p43, Algorithm 1.

**Paper.** Inputs: weight schedule $W \in \mathbb{R}^{T \times N}$, close prices $P \in \mathbb{R}^{T \times N}$, initial capital $C_0 > 0$, cost rate $c \ge 0$. Output: the equity series $E_1, \dots, E_T$. The line numbers are the paper's.

1. $h \leftarrow 0 \in \mathbb{R}^N$ (share holdings)
2. $\mathrm{cash} \leftarrow C_0$
3. for each rebalance date $t = 1, \dots, T$:
4. $\quad V_t \leftarrow \mathrm{cash} + h^{\top} p_t$ (mark to market)
5. $\quad w_t \leftarrow W_{t,:}$ (target weights from the schedule)
6. $\quad \delta \leftarrow w_t \cdot V_t - h \odot p_t$ (trade deltas, in dollars)
7. $\quad \mathrm{cost}_t \leftarrow c \cdot \lVert \delta \rVert_1$ (proportional transaction cost)
8. $\quad V^{net}_t \leftarrow V_t - \mathrm{cost}_t$ (post-cost portfolio value)
9. $\quad h \leftarrow (w_t \cdot V^{net}_t) \oslash p_t$ (reallocate to the target weights)
10. $\quad \mathrm{cash} \leftarrow V^{net}_t - h^{\top} p_t$ (residual cash)
11. $\quad E_t \leftarrow V^{net}_t$
12. end for
13. return $E_1, \dots, E_T$

$p_t$ is row $t$ of $P$; $\odot$ and $\oslash$ are elementwise product and division.

- **Requirement.** R-134 asks for a line-by-line implementation, and R-135 for a module that imports only numpy and pandas.

<a id="ir-2"></a>
### IR-2 Conventions of Algorithm 1

*Implementation Risk* p44.

**Paper.**

- Cost-then-allocate: the cost is computed from the deltas at the pre-cost value $V_t$ and deducted from the total portfolio value before reallocation, so it reduces every position proportionally and is not charged against cash alone.
- Fractional shares are assumed.
- The algorithm shows rebalance days only. On a day without a rebalance the equity is the mark to market, $E_t = \mathrm{cash} + h^{\top} p_t$, with no trading.
- $c$ is a fraction of the traded dollar amount (0.001 is 10 basis points), applied to buys and sells alike.

**Derived.**

- `reference_backtest(weights, prices, capital, cost_rate)` (Arch 5.8) has one row of weights per session, so every session is a rebalance date.
- When the weights of a row sum to less than 1, line 10 leaves the remainder in cash: $\mathrm{cash} = V^{net}_t (1 - 1^{\top} w_t)$.
- The equity returned for date $t$ is the value after that date's cost.

<a id="ir-3"></a>
### IR-3 Worked example of R-134

Not from the paper. **Derived** from IR-1 with the inputs of R-134: weights [[0.5, 0.5], [1, 0]], prices [[10, 20], [11, 20]], capital 1000, cost rate 0.001.

| Line | $t = 1$ | $t = 2$ |
| --- | --- | --- |
| 4: $V_t$ | 1000 | $49.95 \cdot 11 + 24.975 \cdot 20 = 1048.95$ |
| 6: $\delta$ | (500, 500) | $(1048.95 - 549.45, \; 0 - 499.5) = (499.5, -499.5)$ |
| 7: cost | 1.0 | 0.999 |
| 8: $V^{net}_t$ | 999.0 | 1047.951 |
| 9: $h$ | (49.95, 24.975) | (95.268273, 0) |
| 10: cash | 0 | 0 |
| 11: $E_t$ | 999.0 | 1047.951 |

**Checked numerically:** the equity series is [999.0, 1047.951], as R-134 states. At a cost rate of zero it is [1000.0, 1050.0].

## 8. H04: probabilistic and deflated Sharpe ratio (*Deflated Sharpe Ratio*)

<a id="dsr-1"></a>
### DSR-1 Expected maximum Sharpe ratio

*Deflated Sharpe Ratio* p7, eq. 1.

**Paper.** For $N$ independent trials whose Sharpe ratio estimates $\widehat{SR}_n$ are Normal with mean $E[\lbrace \widehat{SR}_n \rbrace]$ and variance $V[\lbrace \widehat{SR}_n \rbrace]$, for $N \gg 1$:

$$E[\max \lbrace \widehat{SR}_n \rbrace] \approx E[\lbrace \widehat{SR}_n \rbrace] + \sqrt{V[\lbrace \widehat{SR}_n \rbrace]} \left( (1 - \gamma) Z^{-1}\left[ 1 - \frac{1}{N} \right] + \gamma Z^{-1}\left[ 1 - \frac{1}{N} e^{-1} \right] \right)$$

$\gamma \approx 0.5772$ is the Euler-Mascheroni constant (0.5772156649 in the paper's code, p13), $Z$ the cumulative distribution function of the standard Normal, $Z^{-1}$ its inverse, and $e$ Euler's number.

- The second argument is $1 - 1/(N e)$.
- **Derived.** The bracket is undefined at $N = 1$ (I3), negative for $1 < N < 1.2836$, and 0.5198 at $N = 2$. `effective_trials` returns non-integer values from 1 upwards, so the negative range can be reached (FQ-3). **Checked numerically.**

<a id="dsr-2"></a>
### DSR-2 Probabilistic and deflated Sharpe ratio

*Deflated Sharpe Ratio* p8, eq. 2 (symbols continue on p9).

**Paper.**

$$\widehat{DSR} \equiv \widehat{PSR}(\widehat{SR}_0) = Z\left[ \frac{(\widehat{SR} - \widehat{SR}_0) \sqrt{T - 1}}{\sqrt{1 - \hat{\gamma}_3 \widehat{SR} + \frac{\hat{\gamma}_4 - 1}{4} \widehat{SR}^2}} \right]$$

$$\widehat{SR}_0 = \sqrt{V[\lbrace \widehat{SR}_n \rbrace]} \left( (1 - \gamma) Z^{-1}\left[ 1 - \frac{1}{N} \right] + \gamma Z^{-1}\left[ 1 - \frac{1}{N} e^{-1} \right] \right)$$

$\widehat{SR}$ is the estimated Sharpe ratio of the selected strategy, $T$ its sample length, $\hat{\gamma}_3$ the skewness and $\hat{\gamma}_4$ the kurtosis of its returns, $V[\lbrace \widehat{SR}_n \rbrace]$ the variance of the Sharpe ratio estimates across the trials, and $N$ the number of independent trials. The probabilistic Sharpe ratio is the same expression with any threshold in place of $\widehat{SR}_0$; the deflated one uses the expected maximum under the null hypothesis that the true Sharpe ratio is zero (DSR-1 with a zero mean).

Conventions:

- **Per period, not annualized.** $\widehat{SR}$, the threshold and $\sqrt{V}$ are Sharpe ratios of one observation period (one session for daily returns). The worked example (DSR-3) divides the annual figures by $\sqrt{250}$, and the annual variance by 250, before applying eq. 2.
- $\hat{\gamma}_4$ is the plain kurtosis, 3 for a Normal distribution, not the excess kurtosis: the paper's Normal case is $\hat{\gamma}_3 = 0$, $\hat{\gamma}_4 = 3$ (p10). pandas `Series.kurt()` and scipy's default `kurtosis` return the excess kurtosis.
- The factor is $\sqrt{T - 1}$.
- The paper does not say which estimators give $\widehat{SR}$, $\hat{\gamma}_3$ and $\hat{\gamma}_4$ (divisor of the standard deviation, bias corrections). See FQ-2.
- **Requirement.** R-199: `probabilistic_sharpe(returns, sr_benchmark)` is eq. 2 for a given threshold, from the sample length and the skewness and kurtosis of the returns. R-202: at a trial count of 1 the deflated ratio equals the probabilistic ratio against zero, which eq. 1 cannot produce by itself.
- In `deflated_sharpe(returns, n_effective, sr_variance)` (Arch 5.9), `sr_variance` must be the variance of per-period Sharpe ratios for eq. 2 to hold (FQ-4).

<a id="dsr-3"></a>
### DSR-3 Worked example of R-200, and the answer to RQ2

*Deflated Sharpe Ratio* p9 (inputs) and p10 (results).

**RQ2: the PDF confirms the inputs.** Page 9 prints, in the analyst's reply: $N = 100$, $V[\lbrace \widehat{SR}_n \rbrace] = \tfrac{1}{2}$, $T = 1250$, $\hat{\gamma}_3 = -3$ and $\hat{\gamma}_4 = 10$, for a strategy with an annualized $\widehat{SR}$ of 2.5 over a daily sample of 5 years. These are the inputs R-200 assumed. The results it expects, 0.9004 at $N = 100$ and 0.9505 at $N = 46$, are both printed on p10.

**Paper** (p10). The variance of one half is of annualized Sharpe ratios; the paper converts to per-period values "with 250 observations per year":

$$\widehat{SR}_0 = \sqrt{\frac{1}{2 \cdot 250}} \left( (1 - \gamma) Z^{-1}\left[ 1 - \frac{1}{100} \right] + \gamma Z^{-1}\left[ 1 - \frac{1}{100} e^{-1} \right] \right) \approx 0.1132$$

$$\widehat{DSR} \approx Z\left[ \frac{\left( \frac{2.5}{\sqrt{250}} - 0.1132 \right) \sqrt{1249}}{\sqrt{1 - (-3) \frac{2.5}{\sqrt{250}} + \frac{10 - 1}{4} \left( \frac{2.5}{\sqrt{250}} \right)^2}} \right] = 0.9004 < 0.95$$

The paper adds that with only $N = 46$ independent trials the result would have been 0.9505, and that with Normal returns ($\hat{\gamma}_3 = 0$, $\hat{\gamma}_4 = 3$) it is 0.9505 after $N = 88$ trials.

**Checked numerically** (standard library only):

| Case | Bracket of DSR-1 | $\widehat{SR}_0$ | Result | Paper |
| --- | --- | --- | --- | --- |
| $N = 100$ | 2.530603 | 0.113172 | 0.900397 | 0.9004 |
| $N = 46$ | 2.244171 | 0.100362 | 0.950502 | 0.9505 |
| $N = 45$ | | | 0.951607 | |
| $N = 47$ | | | 0.949404 | |
| Normal returns, $N = 88$ | | | 0.950491 | 0.9505 |
| Normal returns, $N = 89$ | | | 0.949839 | |

Intermediate values at $N = 100$: per-period $\widehat{SR} = 2.5 / \sqrt{250} = 0.158114$; denominator $\sqrt{1 + 0.474342 + 0.056250} = 1.237171$; argument of $Z$ 1.2838. Putting the rounded 0.1132 into the second formula gives 0.90026, so the printed 0.9004 comes from the unrounded threshold.

**Requirement differs: the example needs 250 periods per year, not 252.** R-198 annualizes with 252 by default and R-200 does not say which factor its test uses. With 252 the same inputs give $\widehat{SR}_0 = 0.112722$, 0.899672 at $N = 100$ and 0.949956 at $N = 46$. Both miss R-200's tolerance of 0.0005 (by 0.0007 and 0.00054), and the second falls below 0.95. `test_paper_example` must therefore feed per-period values obtained with 250: $\widehat{SR} = 2.5 / \sqrt{250}$ and a Sharpe-ratio variance of $0.5 / 250 = 0.002$ (FQ-1).

The paper gives moments, not a return series, while `deflated_sharpe` takes a return series. The example can be evaluated only from the moments; how the test reaches that code path is for the ticket to say.

<a id="dsr-4"></a>
### DSR-4 Effective number of independent trials

*Deflated Sharpe Ratio* p14, eq. 8 and eq. 9.

**Paper.** For $M$ trials with correlation matrix entries $\rho_{i,j}$, the equal-weighted average correlation is

$$\rho = \frac{\sum_{i=1}^{M} \sum_{j=1}^{M} \rho_{i,j} - M}{M (M - 1)} = \frac{2 \sum_{i=1}^{M} \sum_{j=i+1}^{M} \rho_{i,j}}{M (M - 1)}$$

and the implied number of independent trials interpolates between $N \to 1$ as $\rho \to 1$ and $N \to M$ as $\rho \to 0$:

$$\widehat{N} = \hat{\rho} + (1 - \hat{\rho}) M$$

The paper notes that the average correlation of a proper correlation matrix lies in $(-\tfrac{1}{M-1}, 1]$, and warns that with fewer observations than $\tfrac{1}{2} M (M - 1)$ the estimate may itself be overfit (p15).

- **Requirement.** R-201 is eq. 9 with $\hat{\rho}$ the mean off-diagonal correlation of the $M$ trial return columns, limited to the range 1 to $M$. The limit is the requirement's: eq. 9 exceeds $M$ when $\hat{\rho} < 0$, by at most 1 (at $\hat{\rho} = -\tfrac{1}{M-1}$ it gives $M + 1$). **Checked numerically.**

## 9. H05: probability of backtest overfitting (*Probability of Backtest Overfitting*)

<a id="pbo-1"></a>
### PBO-1 Rankings and the definition of PBO

*Probability of Backtest Overfitting* p10, definition 2.1; p11, definition 2.2 and eq. 2.2.

**Paper.** For $N$ strategies, $R = (R_1, \dots, R_N)$ are the in-sample (IS) performances and $\bar{R}$ the out-of-sample (OOS) ones; $r$ and $\bar{r}$ are their rankings. In the paper's example $R^c = (0.5, 1.1, 0.7)$ gives $r^c = (1, 3, 2)$: **rank 1 is the worst and rank $N$ the best.** $\Omega^{\ast}_n$ is the set of rankings in which strategy $n$ has rank $N$, that is, is the best in sample. Then

$$PBO = \sum_{n=1}^{N} \mathrm{Prob}[\bar{r}_n < N/2 \mid r \in \Omega^{\ast}_n] \; \mathrm{Prob}[r \in \Omega^{\ast}_n]$$

In words: the probability that the strategy that is best in sample ranks below the median out of sample.

<a id="pbo-2"></a>
### PBO-2 Combinatorially symmetric cross-validation (Algorithm 2.3)

*Probability of Backtest Overfitting* p12-13, Algorithm 2.3 (eq. 2.3 on p12, eq. 2.4 on p13).

**Paper.**

1. Form the matrix $M$ of order $T \times N$: column $n$ holds the profit-and-loss series of trial $n$ over $t = 1, \dots, T$. The rows must be synchronous across trials, and the performance statistic must be computable on subsamples of a column.
2. Partition $M$ across rows into an even number $S$ of disjoint submatrices $M_s$ of equal dimensions, each of order $(T/S) \times N$.
3. Form all combinations $C_S$ of the $M_s$ taken in groups of $S/2$. Their number is

$$\binom{S}{S/2} = \binom{S-1}{S/2-1} \frac{S}{S/2} = \dots = \prod_{i=0}^{S/2-1} \frac{S - i}{S/2 - i}$$

4. For each combination $c \in C_S$:
   1. Form the training set $J$ by joining the $S/2$ submatrices of $c$ in their original order. $J$ is of order $(T/2) \times N$.
   2. Form the testing set $\bar{J}$ as the complement of $J$ in $M$, also in the original order.
   3. Compute the vector $R^c$ of the $N$ performance statistics on the columns of $J$, and its ranking $r^c$.
   4. Do the same on $\bar{J}$ to get $\bar{R}^c$ and $\bar{r}^c$.
   5. Find $n^{\ast}$, the strategy that is best in sample ($r^c_{n^{\ast}} = N$).
   6. Define the relative rank of its out-of-sample performance, $\bar{\omega}_c$ (below).
   7. Define the logit $\lambda_c$ (below).
5. Collect the $\lambda_c$ over all $c \in C_S$; their relative frequencies are $f(\lambda)$ (below).

$$\bar{\omega}_c = \frac{\bar{r}^c_{n^{\ast}}}{N + 1} \in (0, 1), \qquad \lambda_c = \ln \frac{\bar{\omega}_c}{1 - \bar{\omega}_c}, \qquad f(\lambda) = \sum_{c \in C_S} \frac{\chi_{\lbrace \lambda \rbrace}(\lambda_c)}{\#(C_S)}$$

where $\chi$ is the indicator function and $\#(C_S)$ the number of combinations.

- In step 4.3 the paper's text reads "the nth column of J (the testing set)"; $J$ is the training set, as step 4.1 defines it.
- The divisor is $N + 1$, not $N$, so the relative rank never reaches 0 or 1 and the logit is finite.
- High logits mean that in-sample and out-of-sample performance agree.
- Every combination's complement is itself a combination, so each split is also evaluated with the roles swapped.
- **Requirement.** R-203 ranks by Sharpe ratio. The paper names the Sharpe ratio as an example and leaves the statistic open.
- **Derived.** Within one split every trial has the same number of rows, so the ranking, the logits, PBO, the slope of PBO-4 and the probability of loss do not depend on the divisor of the standard deviation or on annualization. The intercept of PBO-4 does. **Checked numerically** (PBO-6).
- The paper does not say what to do when $T$ is not a multiple of $S$, when two trials tie, or when a trial has zero variance in one half (FQ-5).

<a id="pbo-3"></a>
### PBO-3 The PBO estimate

*Probability of Backtest Overfitting* p14 (section 3.1) and p15.

**Paper.**

$$\phi = \int_{-\infty}^{0} f(\lambda) \, d\lambda$$

This is the rate at which the strategies that are best in sample underperform the median out of sample. A customary rule rejects models whose PBO exceeds 0.05 (p15).

- **Requirement.** R-203 returns as PBO the share of splits whose logit is zero or negative.
- **Requirement fixes what the paper leaves ambiguous.** A logit of exactly zero occurs only for an odd number of trials, when the selected trial's out-of-sample rank is $(N + 1)/2$, the median itself. Eq. 2.2 counts ranks strictly below $N/2$, which excludes it; the integral up to zero can be read either way. R-203 counts it as overfit, the more conservative reading. In the example of PBO-6 the two readings give 0.667 and 0.333 (FQ-6).

<a id="pbo-4"></a>
### PBO-4 Performance degradation and probability of loss

*Probability of Backtest Overfitting* p15 (section 3.2).

**Paper.** Over the pairs $(R^c_{n^{\ast}}, \bar{R}^c_{n^{\ast}})$, one per combination, the regression

$$\bar{R}^c_{n^{\ast}} = \alpha + \beta R^c_{n^{\ast}} + \varepsilon^c$$

measures the degradation; the paper expects $\beta$ to be negative in most practical cases. The probability of loss is the proportion of combinations with a negative out-of-sample performance of the selected strategy, $\mathrm{Prob}[\bar{R}^c_{n^{\ast}} < 0]$.

- **Requirement.** R-206: `PBOResult` holds the PBO, the logits, the slope $\beta$ ("out-of-sample on in-sample performance of the selected trials") and the share of splits in which the selected trial loses out of sample.
- The paper does not name the estimator of the regression; ordinary least squares over the $\binom{S}{S/2}$ points is assumed here.

<a id="pbo-5"></a>
### PBO-5 Number of splits

*Probability of Backtest Overfitting* p12 (eq. 2.3) and p23.

**Paper.** $S = 12$ gives 924 logits and $S = 24$ gives 2,704,156. For $S = 16$ the paper prints 12,780, on both pages.

- **Derived.** $\binom{16}{8} = 12{,}870$. The paper's 12,780 is a misprint: its own eq. 2.3 gives 12,870, and its other two counts match the binomial coefficient. **Checked numerically.** R-203 uses 12,870 (I19).
- The paper bounds the standard deviation of an estimated proportion by $\sqrt{1/(4n)}$ for $n$ logits and quotes less than 0.0045 for $S = 16$; with 12,870 logits it is 0.0044.

<a id="pbo-6"></a>
### PBO-6 Worked example at S = 4

Not from the paper, which has no numeric example of the algorithm. **Derived** with PBO-2 to PBO-4 for $T = 8$, $N = 3$, $S = 4$ (blocks of two rows), ranking by Sharpe ratio (mean over standard deviation, divisor $n - 1$). The example was chosen to have no ties and no zero Sharpe ratio in any split, and to produce all three possible logits.

Returns (rows are observations, columns are trials 1 to 3):

| Row | Block | Trial 1 | Trial 2 | Trial 3 |
| --- | --- | --- | --- | --- |
| 1 | 1 | 3 | -1 | -1 |
| 2 | 1 | 1 | 0 | 0 |
| 3 | 2 | 3 | 3 | -2 |
| 4 | 2 | 0 | 1 | 0 |
| 5 | 3 | 3 | -1 | 3 |
| 6 | 3 | -1 | 3 | 3 |
| 7 | 4 | -1 | -1 | -2 |
| 8 | 4 | 3 | -2 | 1 |

The six splits:

| IS blocks | OOS blocks | IS Sharpe ratios | OOS Sharpe ratios | $n^{\ast}$ | OOS rank | $\bar{\omega}_c$ | $\lambda_c$ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1, 2 | 3, 4 | 1.166667, 0.439155, -0.783349 | 0.433013, -0.112747, 0.529009 | 1 | 2 | 0.50 | 0 |
| 1, 3 | 2, 4 | 0.783349, 0.132068, 0.606339 | 0.606339, 0.112747, -0.500000 | 1 | 3 | 0.75 | 1.098612 |
| 1, 4 | 2, 3 | 0.783349, -1.224745, -0.387298 | 0.606339, 0.783349, 0.408248 | 1 | 2 | 0.50 | 0 |
| 2, 3 | 1, 4 | 0.606339, 0.783349, 0.408248 | 0.783349, -1.224745, -0.387298 | 2 | 1 | 0.25 | -1.098612 |
| 2, 4 | 1, 3 | 0.606339, 0.112747, -0.500000 | 0.783349, 0.132068, 0.606339 | 1 | 3 | 0.75 | 1.098612 |
| 3, 4 | 1, 2 | 0.433013, -0.112747, 0.529009 | 1.166667, 0.439155, -0.783349 | 3 | 1 | 0.25 | -1.098612 |

Results:

| Quantity | Value |
| --- | --- |
| Number of splits | 6 |
| PBO, logits zero or negative (R-203) | 4/6 = 0.666667 |
| PBO, logits strictly negative | 2/6 = 0.333333 |
| Degradation slope $\beta$ | 0.947884 |
| Probability of loss | 2/6 = 0.333333 |

**Checked numerically.** With divisor $n$ instead of $n - 1$ every Sharpe ratio is multiplied by the same factor; the ranks, the logits, PBO, the slope and the probability of loss are unchanged, and only the intercept moves (-0.664778 against -0.767620).

One hand check: trial 1 in sample for blocks 1 and 2 has returns 3, 1, 3, 0, mean 1.75, sum of squared deviations 6.75, standard deviation $\sqrt{6.75/3} = 1.5$, Sharpe ratio 1.166667.

## 10. Beyond PQ-2: Table 4 of *And the Cross-Section* (H07)

H07 is not one of the tasks named in PQ-2, and R-208 states its inputs in full. They came from the text copy, where tables are garbled, so they were checked here as well.

<a id="mt-1"></a>
### MT-1 Bonferroni, Holm and BHY adjustments and Table 4

*And the Cross-Section* p15 (Table 4, Bonferroni), p16-17 (Holm), p17-18 (BHY).

**Paper.** For $M$ tests with p-values $p_i$, ordered $p_{(1)} \le \dots \le p_{(M)}$, at significance level $\alpha_w$ (Bonferroni, Holm) or $\alpha_d$ (BHY):

$$p^{Bonferroni}_i = \min[M p_i, 1] \qquad \text{reject when } p_i \le \frac{\alpha_w}{M}$$

$$p^{Holm}_{(i)} = \min\left[ \max_{j \le i} \lbrace (M - j + 1) p_{(j)} \rbrace, 1 \right] \qquad \text{reject } H_{(1)}, \dots, H_{(k-1)}, \; k \text{ the minimum index with } p_{(k)} > \frac{\alpha_w}{M + 1 - k}$$

$$p^{BHY}_{(M)} = p_{(M)}, \qquad p^{BHY}_{(i)} = \min\left[ p^{BHY}_{(i+1)}, \frac{M \cdot c(M)}{i} p_{(i)} \right] \text{ for } i \le M - 1, \qquad c(M) = \sum_{j=1}^{M} \frac{1}{j}$$

BHY rejects $H_{(1)}, \dots, H_{(k)}$ with $k$ the maximum index such that $p_{(k)} \le \frac{k}{M \cdot c(M)} \alpha_d$.

**Table 4** (p15), ten hypothetical tests, both levels at 5%:

| Test | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| t-ratio | 1.99 | 2.63 | 2.21 | 3.43 | 2.17 | 2.64 | 4.56 | 5.34 | 2.75 | 2.49 |
| p-value (%) | 4.66 | 0.85 | 2.71 | 0.05 | 3.00 | 0.84 | 0.00 | 0.00 | 0.60 | 1.28 |

The paper reports 3 discoveries for Bonferroni (tests 4, 7, 8), 4 for Holm (adding test 9) and 6 for BHY.

- **R-208's inputs and expected sets are confirmed** against the rendered table. **Checked numerically:** with these p-values the three procedures reject {4, 7, 8}, {4, 7, 8, 9} and {2, 4, 6, 7, 8, 9}; $c(10) = 2.928968$.
- The Holm thresholds printed in Panel B (0.50, 0.56, 0.63, 0.71, 0.83, 1.00, 1.25, 1.67, 2.50, 5.00 percent) match $\alpha_w / (M + 1 - k)$. The BHY row of Panel C is printed as 0.15, 0.21, 0.50, 0.70, 0.85, 1.00, 1.20, 1.35, 1.55, 1.70; the formula gives 0.17, 0.34, 0.51, 0.68, 0.85, 1.02, 1.19, 1.37, 1.54, 1.71. The discoveries are the same either way, but a test must not assert the printed BHY thresholds.
- The Holm decision sits close to its threshold (the fifth ordered p-value, 0.84 percent, against 0.8333 percent), so the p-values must be used as printed, not recomputed from the t-ratios. The BHY decision is not close (sixth ordered p-value 0.85 against 1.02; seventh 1.28 against 1.19).

## 11. What was checked numerically

| Item | Check | Result |
| --- | --- | --- |
| SA-2 | $\rho$ of eq. 8 against `numpy.corrcoef` | equal |
| SA-3 | factor returns uncorrelated over the window; variance equals eigenvalue | off-diagonal below 2e-15 with the window's $\bar{\sigma}_i$; up to 0.08 with another volatility |
| SA-4 | $X_{60} = 0$ with an intercept | below 4e-16 |
| SA-6 | bound on $b$ | $e^{-1/30} = 0.967216$ |
| SA-7, SA-9 | both sides of eq. 22; closed forms; effect of the variance divisor | agree; $s = -0.6$; divisor moves $s$ by about 1% |
| TSM-1 | centre of mass; four finite-sample variants | 60; variants differ by up to 1% |
| HRP-1, HRP-6 | Examples 1 to 6 of the paper; weights of the 3-asset case | reproduced; 0.270270, 0.270270, 0.459459 |
| HRP-4 | singular covariance, 50 assets from 30 sessions | weights positive, sum 1 |
| HRP-5 | diagonal covariance against inverse-variance weights | difference 4e-17 |
| MZ-5 | eq. 2.3 times portfolio value equals eq. 2.2 | equal |
| MZ-7 | return term negative for all non-zero weights when $\rho > \lvert \mu \rvert$ | holds on 20,000 random vectors |
| MZ-8 | eq. 17 against the maximum over the uncertainty set; eq. 18; long-only form | equal |
| MZ-11 | analytic solution against scipy SLSQP | equal to 8 digits |
| IR-3 | Algorithm 1 on R-134's inputs | [999.0, 1047.951] |
| DSR-1 | sign of the bracket for small $N$ | negative below $N = 1.2836$ |
| DSR-3 | the paper's example, 250 and 252 periods per year | 0.900397 and 0.950502 with 250; 0.899672 and 0.949956 with 252 |
| DSR-4 | range of eq. 9 | $M + 1$ at the lowest possible $\hat{\rho}$ |
| PBO-5 | binomial coefficients | 924; 12,870; 2,704,156 |
| PBO-6 | all six splits | table in PBO-6 |
| MT-1 | Table 4 discoveries | as R-208 states |

Not checked numerically: MZ-9 as a whole (no convex solver was available for the Python version at hand; MZ-11 used a general optimizer), MZ-10 (a monotonicity property the allocator's own test must establish), and anything on real data.

## 12. Where a requirement and its paper differ

Differences that an implementer could trip over. Nothing here changes `requirements.md`; items that need a decision are repeated in section 13.

| Requirement | Paper | Difference |
| --- | --- | --- |
| R-200 | *Deflated Sharpe Ratio* p9-10 | Inputs confirmed. The example works only with 250 observations per year; R-198's default of 252 misses the tolerance (DSR-3, FQ-1). |
| R-199 | *Deflated Sharpe Ratio* p8-10 | Kurtosis is the plain kurtosis (3 for a Normal). Estimators of the moments are not given (DSR-2, FQ-2). |
| R-201 | *Deflated Sharpe Ratio* p14 | The limit to the range 1 to $M$ is the requirement's; eq. 9 can reach $M + 1$ (DSR-4). |
| R-202 | *Deflated Sharpe Ratio* p7 | The requirement defines $N = 1$; eq. 1 is also negative for $1 < N < 1.2836$ (DSR-1, FQ-3). |
| R-203 | *Probability of Backtest Overfitting* p11, p14 | A logit of zero is counted as overfit; eq. 2.2 is strict (PBO-3, FQ-6). 12,870 splits against the printed 12,780 (PBO-5). |
| R-065 | *Statistical Arbitrage* p45-46 | No numeric fixture exists in the PDF to transcribe. Intercept, variance divisor and the set averaged are not fixed (SA-4 to SA-9, FQ-7). |
| R-066 | *Statistical Arbitrage* p46 | The paper's formulas need $0 < b$; the requirement names only slow reversion (SA-6, FQ-7). |
| R-067 | *Statistical Arbitrage* p4, p20 | One exit level of 0.5 for both sides; eq. 16 closes short positions at 0.75 (SA-8, FQ-13). |
| R-300 | *Statistical Arbitrage* p6-9 | "Volatility" must be the window's sample standard deviation for the factors to be uncorrelated; window length in returns or prices not stated (SA-3, FQ-8). |
| R-089 | *Time Series Momentum* p13 | The paper annualizes with 261, sums an infinite history and subtracts the exponentially weighted mean; the requirement is silent on all three (TSM-1, FQ-9). |
| R-095, R-096 | *Multi-Period Trading* p12 | The asymmetric term $c x$ of eq. 2.2 is dropped; volatility is a return volatility as in *Markowitz at Seventy* eq. 10 (MZ-5). |
| R-109 | *Markowitz at Seventy* p19, p24 | The formula is not in the paper; it is derived, and holds only when every other constraint is slack and the risk target is in the units of the covariance (MZ-11, FQ-10). |
| R-110 | *Markowitz at Seventy* p23 | No $r^{rf} c$ term; `rho_scale` is not in the paper (MZ-7). |
| R-111 | *Markowitz at Seventy* p24 | Eq. 17 gives the square of the worst-case risk; the diagnostic's form and units are not stated (MZ-8, FQ-11). |
| R-117, R-118 | *Building Diversified Portfolios* p5-8 | Clustering runs on the distance of distances, with single linkage. The requirements' tests cannot tell this from clustering on $d$ (HRP-1, FQ-12). |
| R-134 | *Implementation Risk* p43-44 | None. The worked example is the requirement's own and is reproduced (IR-3). |
| R-208 | *And the Cross-Section* p15 | None in inputs or expected sets. The BHY threshold row printed in Panel C does not match the formula (MT-1). |

## 13. Open questions

A question is listed when neither the paper nor `requirements.md` fixes a detail and the detail changes a number that an acceptance check compares. Until answered, the implementer of the task must state the convention chosen in the module's docstring and in the test, and report it.

- **FQ-1 (H04, R-200). Periods per year in the worked example.** The example reproduces only with 250. Confirm that `test_paper_example` passes per-period inputs computed with 250 ($\widehat{SR} = 2.5 / \sqrt{250}$, variance $0.5 / 250$), independently of the default of R-198. Also say how the test reaches `deflated_sharpe`, whose signature takes a return series while the paper gives only moments.
- **FQ-2 (H04, R-199). Estimators of the moments.** Which divisor does the standard deviation in $\widehat{SR}$ use, and are skewness and kurtosis the plain moment ratios or bias-corrected? R-199's check is to 1e-10 "against the formula evaluated from sample moments", so the test and the code must share one definition. The paper is silent.
- **FQ-3 (H04, H14, R-202). Trial counts between 1 and 2.** `effective_trials` returns non-integers from 1 upwards. For $1 < N < 1.2836$ the bracket of eq. 1 is negative, the threshold falls below zero, and the deflated ratio exceeds the probabilistic ratio against zero; it tends to 1 as $N$ approaches 1 from above, while R-202 sets it equal to the probabilistic ratio at exactly 1. Should the threshold be floored at zero, should $N$ be rounded up, or is the formula applied as it stands?
- **FQ-4 (H14, R-217). Variance of the trial Sharpe ratios.** Confirm that `sr_variance` is the variance of per-period (not annualized) Sharpe ratios, and say which divisor it uses. With few trials the divisor matters.
- **FQ-5 (H05, R-203, R-206, R-207). Rows, ties and undefined statistics.** (a) When $T$ is not a multiple of $S$, which rows are dropped? The benchmark of R-207 has 2,740 rows and $S = 16$, which leaves 4 over. (b) How is $n^{\ast}$ chosen when two trials tie in sample, and what rank does a tie get out of sample? (c) What is the Sharpe ratio of a trial with zero variance in one half (for example a trial that held only cash)?
- **FQ-6 (H05, R-203). A logit of exactly zero.** R-203 counts it as overfit; eq. 2.2 of the paper does not. Confirm the requirement's reading. It only matters for an odd number of trials.
- **FQ-7 (C04, R-065, R-066). Conventions of the s-score.** (a) There is no numeric example in the PDF; confirm that the fixture is computed from SA-4 to SA-7. (b) Confirm the intercept in the 60-session regression on the factors. (c) Divisor of $\mathrm{Variance}(\zeta)$: 59, 58 or 57. (d) Which stocks enter the cross-sectional average of $a / (1 - b)$: the whole universe, or only those that pass the speed filter. (e) What is returned when $b \le 0$ or $b \ge 1$.
- **FQ-8 (C03, R-300). Window of the correlation matrix.** 252 returns (253 prices), as $M$ is in the paper, or 252 prices (251 returns)? Also confirm that the volatility in the eigenportfolio weights is the sample standard deviation of that window and not the estimator of R-089 (PL-9 injects that estimator into `tsmom` and `pv_characteristics` only).
- **FQ-9 (D01, R-089). Form of the EWMA volatility.** (a) Is the estimator's output per session or annual, and if annual with which factor (the paper uses 261, R-198 uses 252)? (b) Are the weights renormalized over a finite history (pandas `ewm(com=60, adjust=True)`), or is the infinite sum truncated? (c) Is the exponentially weighted mean subtracted, as in the paper? (d) Is there a minimum number of returns below which the result is NaN?
- **FQ-10 (E03, R-106, R-109). Units of the risk target.** `risk_target_annual` is annual and the risk estimate is per session. Confirm the conversion: $\sigma^{tar}$ is `risk_target_annual` divided by $\sqrt{252}$, with the periods per year of R-198.
- **FQ-11 (E04, R-111). The `sigma_wc` diagnostic.** Is it $\sigma^{wc}$ (eq. 18) or its square (eq. 17), and per session or annual?
- **FQ-12 (E02, R-117). HRP edge cases.** (a) Confirm that the clustering uses the distance of distances $\tilde{d}$, as the paper does. (b) What does the allocator do with an asset of zero variance, for which the correlation is undefined and the inverse variance infinite?
- **FQ-13 (C05, R-067). Exit bands.** Keep one exit level of 0.5 for both sides (the paper's p4), or close short positions at 0.75 and long positions at 0.50 (the paper's eq. 16)?

This document needs human approval before the tickets of C03, C04, D01, E02, E03, E04, F01, H04 and H05 cite it.
