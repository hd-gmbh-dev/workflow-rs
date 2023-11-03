export default {
    transform: {
        '^.+\\.ts?$': ['ts-jest', {}],
        '\\.js$': ['babel-jest', { configFile: './babel.config.json' }],
    },
};
