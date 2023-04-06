
const margin = 10;
// TODO responsive layout
const width = 500 - 2 * margin;
const height = 200 - 2 * margin;

// TODO can we not hardcode this?
const num_items = 64;
const item_domain = [];
for (let i = 0; i < num_items; i++) { item_domain.push(i) }

const scales = {};
const charts = {};

// TODO debug code, remove
const sample = [0, 99, 20, 3, 4, 5, 60, 7, 0, 99, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 99, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7];

function init_chart(id) {
    if (charts[id] != null) { return; }
    const svg = d3.select('#bar-chart-' + id);
    const chart = svg.append('g')
        .attr('transform', `translate(${margin}, ${margin})`);

    const xScale = d3.scaleBand()
        .range([0, width])
        .domain(item_domain)
        .padding(0.4)

    const yScale = d3.scaleLinear()
        .range([height, 0])
        .domain([0, 255]);

    scales[id] = { 'x': xScale, 'y': yScale };
    charts[id] = chart;

}


function update(id, data) {
    const xScale = scales[id]['x'];
    const yScale = scales[id]['y'];

    const chart = charts[id];

    const barGroups = chart.selectAll('rect')
        .data(data);

    barGroups
        .join('rect')
        .attr('class', 'bar')
        .attr('x', (g, i) => xScale(i))
        .attr('y', (v) => yScale(v))
        .attr('height', (g) => height - yScale(g))
        .attr('width', xScale.bandwidth());
}
