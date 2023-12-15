const mix = require("laravel-mix");

mix
  .js("src/js/app.js", "js")
  .js("src/js/libs/components.js", "js/libs")
  .js("src/js/libs/forms.js", "js/libs")
  .postCss("src/css/app.css", "css", [
    require("tailwindcss"),
    require("autoprefixer"),
  ])
  .options({ processCssUrls: false })
  .webpackConfig({
    module: {
      rules: [
        {
          test: /\.js$/,
          enforce: "pre",
          use: ["source-map-loader"],
        },
      ],
    },
    devServer: {
      open: true,
    },
  })
  .copyDirectory("src/html/", "dist")
  .copyDirectory("src/images", "dist/images")
  .copyDirectory("src/fonts", "dist/fonts")
  .setPublicPath("dist")
  .disableNotifications();

mix
  .js("src/js/pages/elements-alert.js", "js/pages")

  .js("src/js/pages/components-accordion.js", "js/pages")
  .js("src/js/pages/components-collapse.js", "js/pages")
  .js("src/js/pages/components-tab.js", "js/pages")
  .js("src/js/pages/components-dropdown.js", "js/pages")
  .js("src/js/pages/components-popover.js", "js/pages")
  .js("src/js/pages/components-modal.js", "js/pages")
  .js("src/js/pages/components-drawer.js", "js/pages")
  .js("src/js/pages/components-treeview.js", "js/pages")
  .js("src/js/pages/components-table-advanced.js", "js/pages")
  .js("src/js/pages/components-table-gridjs.js", "js/pages")
  .js("src/js/pages/components-apexcharts.js", "js/pages")
  .js("src/js/pages/components-carousel.js", "js/pages")
  .js("src/js/pages/components-clipboard.js", "js/pages")
  .js("src/js/pages/components-monochrome.js", "js/pages")

  .js("src/js/pages/forms-datepicker.js", "js/pages")
  .js("src/js/pages/forms-datetimepicker.js", "js/pages")
  .js("src/js/pages/forms-timepicker.js", "js/pages")
  .js("src/js/pages/forms-input-mask.js", "js/pages")
  .js("src/js/pages/forms-text-editor.js", "js/pages")
  .js("src/js/pages/forms-tom-select.js", "js/pages")
  .js("src/js/pages/forms-upload.js", "js/pages")
  .js("src/js/pages/forms-layout-2.js", "js/pages")
  .js("src/js/pages/forms-layout-3.js", "js/pages")
  .js("src/js/pages/forms-layout-4.js", "js/pages")

  .js("src/js/pages/pages-card-user-1.js", "js/pages")
  .js("src/js/pages/pages-card-user-2.js", "js/pages")
  .js("src/js/pages/pages-card-user-3.js", "js/pages")
  .js("src/js/pages/pages-card-user-4.js", "js/pages")
  .js("src/js/pages/pages-card-user-5.js", "js/pages")
  .js("src/js/pages/pages-card-user-6.js", "js/pages")
  .js("src/js/pages/pages-card-user-7.js", "js/pages")
  .js("src/js/pages/pages-card-blog-1.js", "js/pages")
  .js("src/js/pages/pages-card-blog-2.js", "js/pages")
  .js("src/js/pages/pages-card-blog-3.js", "js/pages")
  .js("src/js/pages/pages-card-blog-4.js", "js/pages")
  .js("src/js/pages/pages-card-blog-5.js", "js/pages")
  .js("src/js/pages/pages-card-blog-6.js", "js/pages")
  .js("src/js/pages/pages-card-blog-7.js", "js/pages")
  .js("src/js/pages/pages-card-blog-8.js", "js/pages")
  .js("src/js/pages/pages-blog-details.js", "js/pages")
  .js("src/js/pages/pages-help-1.js", "js/pages")
  .js("src/js/pages/pages-help-2.js", "js/pages")
  .js("src/js/pages/pages-help-3.js", "js/pages")
  .js("src/js/pages/pages-sign-in-2.js", "js/pages")
  .js("src/js/pages/pages-sign-up-2.js", "js/pages")
  .js("src/js/pages/pages-error-404-1.js", "js/pages")
  .js("src/js/pages/pages-error-404-2.js", "js/pages")
  .js("src/js/pages/pages-error-404-3.js", "js/pages")
  .js("src/js/pages/pages-error-404-4.js", "js/pages")
  .js("src/js/pages/pages-error-429.js", "js/pages")

  .js("src/js/pages/apps-chat.js", "js/pages")
  .js("src/js/pages/apps-filemanager.js", "js/pages")
  .js("src/js/pages/apps-kanban.js", "js/pages")
  .js("src/js/pages/apps-mail.js", "js/pages")
  .js("src/js/pages/apps-nft-1.js", "js/pages")
  .js("src/js/pages/apps-nft-2.js", "js/pages")
  .js("src/js/pages/apps-pos.js", "js/pages")
  .js("src/js/pages/apps-todo.js", "js/pages")
  .js("src/js/pages/apps-travel.js", "js/pages")

  .js("src/js/pages/dashboards-authors.js", "js/pages")
  .js("src/js/pages/dashboards-banking-1.js", "js/pages")
  .js("src/js/pages/dashboards-banking-2.js", "js/pages")
  .js("src/js/pages/dashboards-cms-analytics.js", "js/pages")
  .js("src/js/pages/dashboards-crm-analytics.js", "js/pages")
  .js("src/js/pages/dashboards-crypto-1.js", "js/pages")
  .js("src/js/pages/dashboards-crypto-2.js", "js/pages")
  .js("src/js/pages/dashboards-doctor.js", "js/pages")
  .js("src/js/pages/dashboards-education.js", "js/pages")
  .js("src/js/pages/dashboards-employees.js", "js/pages")
  .js("src/js/pages/dashboards-influencer.js", "js/pages")
  .js("src/js/pages/dashboards-orders.js", "js/pages")
  .js("src/js/pages/dashboards-personal.js", "js/pages")
  .js("src/js/pages/dashboards-teacher.js", "js/pages")
  .js("src/js/pages/dashboards-travel.js", "js/pages")
  .js("src/js/pages/dashboards-widget-contacts.js", "js/pages")
  .js("src/js/pages/dashboards-widget-ui.js", "js/pages")
  .js("src/js/pages/dashboards-workspaces.js", "js/pages")

  .js("src/js/pages/navigation-horizontal.js", "js/pages")

  .setPublicPath("dist")
  .disableNotifications();
