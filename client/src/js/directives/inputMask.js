import Cleave from 'cleave.js/dist/cleave.min'

export default (el, { expression}, {evaluateLater, effect}) => {
    const getContent = evaluateLater(expression)

    effect(() => {
        getContent(content => {
            if (typeof content == 'object')
                el.__x_cleave = new Cleave(el, content)
            else console.warn('Input mask config should be object')
        })
    })
}
