// spell-checker:ignore hljs
function progressBar(totals) {
	const bar = document.createElement("div");
	bar.className = "progress-bar";
	let totalTests = 0;
	for (const [key, value] of Object.entries(totals)) {
		totalTests += value;
	}
	const passPercentage = Math.round(100 * totals["PASS"] / totalTests);
	const skipPercentage = passPercentage + Math.round(100 * totals["SKIP"] / totalTests);

	// The ternary expressions are used for some edge-cases where there are no failing test,
	// but still a red (or beige) line shows up because of how CSS draws gradients.
	bar.style = `background: linear-gradient(
        to right,
        var(--PASS) ${passPercentage}%`
		+ ( passPercentage === 100 ? ", var(--PASS)" :
        `, var(--SKIP) ${passPercentage}%,
        var(--SKIP) ${skipPercentage}%`
		)
        + (skipPercentage === 100 ? ")" : ", var(--FAIL) 0)");
	
	const progress = document.createElement("div");
	progress.className = "progress"
	progress.innerHTML = `
		<span class="counts">
		<span class="PASS">${totals["PASS"]}</span>
		/
		<span class="SKIP">${totals["SKIP"]}</span>
		/
		<span class="FAIL">${totals["FAIL"] + totals["ERROR"]}</span>
		</span>
	`;
	progress.appendChild(bar);
	return progress
}

function parse_result(parent, obj) {
	const totals = {
		PASS: 0,
		SKIP: 0,
		FAIL: 0,
		ERROR: 0,
	};
	for (const [category, content] of Object.entries(obj)) {
		if (typeof content === "string") {
			const p = document.createElement("p");
			p.className = "result-line";
			totals[content]++;
			p.innerHTML = `<span class="result" style="color: var(--${content})">${content}</span> ${category}`;
			parent.appendChild(p);
		} else {
			const categoryName = document.createElement("code");
			categoryName.innerHTML = category;
			categoryName.className = "hljs";

			const details = document.createElement("details");
			const subtotals = parse_result(details, content);
			for (const [subtotal, count] of Object.entries(subtotals)) {
				totals[subtotal] += count;
			}
			const summaryDiv = document.createElement("div");
			summaryDiv.className = "testSummary";
			summaryDiv.appendChild(categoryName);
			summaryDiv.appendChild(progressBar(subtotals));

			const summary = document.createElement("summary");
			summary.appendChild(summaryDiv);

			details.appendChild(summary);
			parent.appendChild(details);
		}
	}
	return totals;
}

fetch("https://raw.githubusercontent.com/uutils/coreutils-tracking/main/gnu-full-result.json")
	.then((r) => r.json())
	.then((obj) => {
		let parent = document.getElementById("test-cov");
		parse_result(parent, obj);
	});
