from django.contrib import admin
from .models import Opening


class OpeningAdmin(admin.ModelAdmin):
    list_display = ("title", "created_by", "created_at")
    ordering = ("created_at",)


admin.site.register(Opening, OpeningAdmin)
