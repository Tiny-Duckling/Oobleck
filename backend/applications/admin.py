from django.contrib import admin
from .models import ApplicationRequest


class ApplicationRequestAdmin(admin.ModelAdmin):
    list_display = ('job_title', 'created_by', 'created_at')
    ordering = ('created_at',)


admin.site.register(ApplicationRequest, ApplicationRequestAdmin)
