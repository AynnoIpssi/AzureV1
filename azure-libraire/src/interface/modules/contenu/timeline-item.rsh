<container.az-timeline-item.{{class}}>
    <container.az-timeline-dot><!container>
    <container.az-timeline-body>
        <container.az-timeline-head>
            <text.az-timeline-title>{{titre}}<!text>
            <if.date != ""><text.az-timeline-date>{{date}}<!text><!if>
        <!container>
        <slot/>
    <!container>
<!container>
